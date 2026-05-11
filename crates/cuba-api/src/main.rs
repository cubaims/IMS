use anyhow::{Context, Result};
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use cuba_shared::{AppState, Settings};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, layer::Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupCommand {
    Serve,
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 整个进程仅在此处加载一次 .env;Settings::from_env() 内部不再做 dotenv。
    dotenvy::dotenv().ok();
    init_tracing();

    let command = startup_command_from_args()?;
    let settings = Settings::from_env()?;

    // 数据库连接池
    let pool = PgPoolOptions::new()
        .max_connections(settings.db_max_conn)
        .min_connections(settings.db_min_conn)
        .acquire_timeout(Duration::from_secs(settings.db_acquire_timeout_secs))
        .idle_timeout(Some(Duration::from_secs(settings.db_idle_timeout_secs)))
        .max_lifetime(Some(Duration::from_secs(settings.db_max_lifetime_secs)))
        .test_before_acquire(true)
        .connect(&settings.database_url)
        .await
        .context("connecting to PostgreSQL")?;

    if command == StartupCommand::Migrate {
        run_migrations(&pool).await?;
        tracing::info!("database migrations finished; exiting");
        return Ok(());
    }

    if settings.run_migrations {
        run_migrations(&pool).await?;
    } else {
        tracing::info!(
            "database migrations skipped; run `cargo run -p cuba-api -- migrate` or set RUN_MIGRATIONS=true to apply them"
        );
    }

    let repo = std::sync::Arc::new(
        cuba_master_data::infrastructure::PostgresMasterDataRepository::new(pool.clone()),
    );

    let master_data_service =
        std::sync::Arc::new(cuba_master_data::application::MasterDataService::new(
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo,
        ));

    let state = AppState {
        db_pool: pool,
        jwt_secret: settings.jwt_secret,
        jwt_issuer: settings.jwt_issuer,
        jwt_expires_seconds: settings.jwt_expires_seconds,
        jwt_refresh_expires_seconds: settings.jwt_refresh_expires_seconds,
    };

    let app = cuba_api::build_router(state).layer(axum::middleware::from_fn({
        let master_data_service = master_data_service.clone();

        move |mut req: Request<Body>, next: Next| {
            let service = master_data_service.clone();
            async move {
                req.extensions_mut().insert(service);
                next.run(req).await
            }
        }
    }));

    let listener = TcpListener::bind(settings.bind_addr.as_str())
        .await
        .with_context(|| format!("binding {}", settings.bind_addr))?;

    tracing::info!(
        "🚀 cuba-api listening on http://{} (env={}, db_max_conn={})",
        settings.bind_addr,
        std::env::var("APP_ENV").unwrap_or_else(|_| "development".into()),
        settings.db_max_conn
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum server error")?;

    tracing::info!("👋 server shut down cleanly");
    Ok(())
}

fn startup_command_from_args() -> Result<StartupCommand> {
    let mut args = std::env::args().skip(1);
    let command = args.next();

    if let Some(extra) = args.next() {
        anyhow::bail!("unexpected extra cuba-api argument: {extra}");
    }

    match command.as_deref() {
        None => Ok(StartupCommand::Serve),
        Some("migrate") => Ok(StartupCommand::Migrate),
        Some(command) => anyhow::bail!("unknown cuba-api command: {command}"),
    }
}

async fn run_migrations(pool: &sqlx::PgPool) -> Result<()> {
    ensure_migration_baseline_safe(pool).await?;
    tracing::info!("running database migrations");
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .context("running migrations")
}

async fn ensure_migration_baseline_safe(pool: &sqlx::PgPool) -> Result<()> {
    let has_ims_schema = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.schemata
            WHERE schema_name IN ('mdm', 'wms', 'rpt', 'sys')
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .context("checking IMS schema state before migration")?;

    if !has_ims_schema {
        return Ok(());
    }

    let has_sqlx_migration_table = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = '_sqlx_migrations'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .context("checking SQLx migration table before migration")?;

    if !has_sqlx_migration_table {
        anyhow::bail!(
            "IMS schemas already exist but SQLx baseline migration 0001 is not recorded; refusing to run migrations because 0001 rebuilds schemas"
        );
    }

    let baseline_recorded = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM public._sqlx_migrations
            WHERE version = 1
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .context("checking SQLx baseline migration before migration")?;

    if !baseline_recorded {
        anyhow::bail!(
            "IMS schemas already exist but SQLx baseline migration 0001 is not recorded; refusing to run migrations because 0001 rebuilds schemas"
        );
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("cuba_api=info,cuba_auth=info,sqlx=warn,tower_http=info")
    });

    let use_json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or_else(|_| !cfg!(debug_assertions));

    let fmt_layer = if use_json {
        tracing_subscriber::fmt::layer().json().boxed()
    } else {
        tracing_subscriber::fmt::layer().pretty().boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c    => tracing::info!("SIGINT received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
