use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use anyhow::{Context, Result};
use cuba_shared::{AppState, Settings};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{EnvFilter, layer::Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let settings = Settings::from_env();

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

    // 运行迁移
    if std::env::var("RUN_MIGRATIONS")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
    {
        tracing::info!("running database migrations…");
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .context("running migrations")?;
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

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.port));

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;

    tracing::info!(
        "🚀 cuba-api listening on http://{addr} (env={}, db_max_conn={})",
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
