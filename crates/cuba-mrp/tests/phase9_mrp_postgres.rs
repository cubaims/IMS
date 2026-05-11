use std::{env, error::Error, fs};

use cuba_mrp::{
    application::{
        CancelMrpSuggestionCommand, CancelMrpSuggestionUseCase, ConfirmMrpSuggestionCommand,
        ConfirmMrpSuggestionUseCase, MrpRunQuery, MrpRunRepository, MrpSuggestionQuery,
        MrpSuggestionRepository, RunMrpCommand, RunMrpUseCase,
    },
    domain::{MaterialId, MrpSuggestionId, MrpSuggestionStatus, Operator, ProductVariantId},
    infrastructure::{PostgresMrpIdGenerator, PostgresMrpStore},
};
use cuba_shared::{PageQuery, write_audit_event};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::test]
async fn phase9_mrp_run_suggestions_status_and_audit_on_real_postgres() -> TestResult<()> {
    let Some(pool) = maybe_pool().await? else {
        return Ok(());
    };

    assert_phase9_mrp_schema_contract(&pool).await?;

    let store = PostgresMrpStore::new(pool.clone());
    let variant_code = active_variant_code(&pool).await?;
    let base_material_id = base_material_id_for_variant(&pool, &variant_code).await?;

    let use_case = RunMrpUseCase::new(
        store.clone(),
        store.clone(),
        store.clone(),
        PostgresMrpIdGenerator,
    );

    let output = use_case
        .execute(RunMrpCommand {
            material_id: Some(MaterialId::new(base_material_id)),
            product_variant_id: Some(ProductVariantId::new(variant_code)),
            demand_qty: Decimal::from(20),
            demand_date: OffsetDateTime::now_utc() + Duration::days(14),
            created_by: Operator::new("phase9-db-test"),
            remark: Some("phase9 integration mrp".to_string()),
        })
        .await?;

    let run_page = MrpRunRepository::list(
        &store,
        MrpRunQuery {
            page: PageQuery {
                page: 1,
                page_size: 10,
            },
            status: None,
            material_id: None,
            product_variant_id: output.product_variant_id.clone(),
            date_from: None,
            date_to: None,
        },
    )
    .await?;

    if !run_page.items.iter().any(|run| run.id == output.run_id) {
        return Err(test_error("MRP run was not returned by repository list"));
    }

    let suggestion_page = MrpSuggestionRepository::list(
        &store,
        MrpSuggestionQuery {
            page: PageQuery {
                page: 1,
                page_size: 20,
            },
            run_id: Some(output.run_id.clone()),
            suggestion_type: None,
            status: None,
            material_id: None,
            required_date_from: None,
            required_date_to: None,
            only_shortage: None,
        },
    )
    .await?;

    if suggestion_page.items.is_empty() {
        return Err(test_error("MRP run did not generate suggestions"));
    }

    let suggestion_id = suggestion_page.items[0].id.clone();

    let confirm_use_case = ConfirmMrpSuggestionUseCase::new(store.clone());
    let confirmed = confirm_use_case
        .execute(ConfirmMrpSuggestionCommand {
            suggestion_id: suggestion_id.clone(),
            confirmed_by: Operator::new("phase9-db-test"),
            remark: Some("confirmed by phase9 integration".to_string()),
        })
        .await?;

    if confirmed.status != MrpSuggestionStatus::Confirmed {
        return Err(test_error("MRP suggestion was not confirmed"));
    }

    let cancel_use_case = CancelMrpSuggestionUseCase::new(store.clone());
    let cancelled = cancel_use_case
        .execute(CancelMrpSuggestionCommand {
            suggestion_id: suggestion_id.clone(),
            cancelled_by: Operator::new("phase9-db-test"),
            reason: "cancelled by phase9 integration".to_string(),
        })
        .await?;

    if cancelled.status != MrpSuggestionStatus::Cancelled {
        return Err(test_error("MRP suggestion was not cancelled"));
    }

    let saved =
        MrpSuggestionRepository::find_by_id(&store, &MrpSuggestionId::new(suggestion_id.as_str()))
            .await?
            .ok_or_else(|| test_error("MRP suggestion disappeared after status changes"))?;

    if saved.status != MrpSuggestionStatus::Cancelled {
        return Err(test_error("MRP suggestion status was not persisted"));
    }

    write_audit_event(
        &pool,
        Some(Uuid::nil()),
        "MRP_RUN",
        Some("wms.wms_mrp_runs"),
        Some(output.run_id.as_str()),
        Some(serde_json::json!({
            "source": "phase9-postgres-test",
            "run_id": output.run_id.as_str()
        })),
    )
    .await;

    let audit_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM sys.sys_audit_log
            WHERE action = 'MRP_RUN'
              AND table_name = 'wms.wms_mrp_runs'
              AND record_id = $1
        )
        "#,
    )
    .bind(output.run_id.as_str())
    .fetch_one(&pool)
    .await?;

    if !audit_exists {
        return Err(test_error("MRP audit event was not written"));
    }

    Ok(())
}

async fn maybe_pool() -> TestResult<Option<PgPool>> {
    if env::var("PHASE9_RUN_DB_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping Phase 9 PostgreSQL MRP tests; set PHASE9_RUN_DB_TESTS=1 to run");
        return Ok(None);
    }

    let Some(database_url) = database_url() else {
        eprintln!("skipping Phase 9 PostgreSQL MRP tests; DATABASE_URL is not set");
        return Ok(None);
    };

    Ok(Some(
        PgPoolOptions::new()
            .max_connections(4)
            .connect(&database_url)
            .await?,
    ))
}

fn database_url() -> Option<String> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }

    let dotenv = fs::read_to_string(".env").ok()?;
    dotenv.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with("DATABASE_URL=") {
            return None;
        }

        Some(
            trimmed
                .trim_start_matches("DATABASE_URL=")
                .trim_matches('"')
                .trim_matches('\'')
                .to_string(),
        )
    })
}

async fn assert_phase9_mrp_schema_contract(pool: &PgPool) -> TestResult<()> {
    let missing: Vec<String> = sqlx::query_scalar(
        r#"
        WITH required(table_schema, table_name, column_name) AS (
            VALUES
                ('wms', 'wms_mrp_runs', 'run_id'),
                ('wms', 'wms_mrp_runs', 'variant_code'),
                ('wms', 'wms_mrp_runs', 'demand_qty'),
                ('wms', 'wms_mrp_runs', 'demand_date'),
                ('wms', 'wms_mrp_runs', 'status'),
                ('wms', 'wms_mrp_suggestions', 'id'),
                ('wms', 'wms_mrp_suggestions', 'run_id'),
                ('wms', 'wms_mrp_suggestions', 'material_id'),
                ('wms', 'wms_mrp_suggestions', 'shortage_qty'),
                ('wms', 'wms_mrp_suggestions', 'suggested_order_type'),
                ('wms', 'wms_mrp_suggestions', 'remarks')
        )
        SELECT required.table_schema || '.' || required.table_name || '.' || required.column_name
        FROM required
        WHERE NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = required.table_schema
              AND table_name = required.table_name
              AND column_name = required.column_name
        )
        ORDER BY 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !missing.is_empty() {
        return Err(test_error(format!(
            "Phase 9 MRP schema contract is missing: {}",
            missing.join(", ")
        )));
    }

    let fn_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            WHERE n.nspname = 'wms'
              AND p.proname = 'fn_run_mrp'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !fn_exists {
        return Err(test_error("wms.fn_run_mrp is missing"));
    }

    Ok(())
}

async fn active_variant_code(pool: &PgPool) -> TestResult<String> {
    let row = sqlx::query(
        r#"
        SELECT pv.variant_code
        FROM mdm.mdm_product_variants pv
        WHERE pv.status = '正常'
          AND EXISTS (
              SELECT 1
              FROM mdm.mdm_bom_components bc
              WHERE bc.bom_id = pv.bom_id
          )
        ORDER BY pv.variant_code
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    row.map(|row| row.get::<String, _>("variant_code"))
        .ok_or_else(|| test_error("no active product variant with BOM components found"))
}

async fn base_material_id_for_variant(pool: &PgPool, variant_code: &str) -> TestResult<String> {
    let row = sqlx::query(
        r#"
        SELECT base_material_id
        FROM mdm.mdm_product_variants
        WHERE variant_code = $1
        "#,
    )
    .bind(variant_code)
    .fetch_optional(pool)
    .await?;

    row.map(|row| row.get::<String, _>("base_material_id"))
        .ok_or_else(|| test_error("product variant base material was not found"))
}

fn test_error(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    Box::new(std::io::Error::other(message.into()))
}
