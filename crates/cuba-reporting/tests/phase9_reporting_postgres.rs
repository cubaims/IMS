use std::{env, error::Error, fs};

use cuba_reporting::{
    application::{
        ExportReportUseCase, GetReportUseCase, RefreshMaterializedViewsCommand,
        RefreshMaterializedViewsUseCase,
    },
    domain::{
        BatchStockSummaryReportFilter, BinStockSummaryReportFilter, CurrentStockReportFilter,
        DataConsistencyReportFilter, InventoryValueReportFilter, LowStockAlertReportFilter,
        MrpShortageReportFilter, QualityStatusReportFilter, ReportExportFormat,
        ReportExportRequest, ReportFilters, ReportQuery, ReportType, StockByZoneReportFilter,
    },
    infrastructure::PostgresReportingRepository,
};
use cuba_shared::{AppState, PageQuery};
use sqlx::{PgPool, postgres::PgPoolOptions};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::test]
async fn phase9_reports_refresh_consistency_and_csv_export_on_real_postgres() -> TestResult<()> {
    let Some(pool) = maybe_pool().await? else {
        return Ok(());
    };

    assert_phase9_reporting_schema_contract(&pool).await?;

    let repository = PostgresReportingRepository::new(test_state(pool));
    let get_report = GetReportUseCase::new(repository.clone());
    let refresh = RefreshMaterializedViewsUseCase::new(repository.clone());
    let export = ExportReportUseCase::new(repository);

    let refresh_result = refresh
        .execute(RefreshMaterializedViewsCommand {
            mode: "all".to_string(),
            concurrently: true,
            remark: Some("phase9 integration refresh".to_string()),
        })
        .await?;

    if !refresh_result.refreshed || refresh_result.views.len() != 8 {
        return Err(test_error(
            "report materialized view refresh did not cover 8 views",
        ));
    }

    for report_type in phase9_report_types() {
        let page = get_report
            .execute(ReportQuery {
                report_type,
                filters: empty_filters(report_type),
                page: PageQuery {
                    page: 1,
                    page_size: 5,
                },
            })
            .await?;

        if page.page != 1 || page.page_size != 5 {
            return Err(test_error(format!(
                "{report_type:?} did not preserve pagination metadata"
            )));
        }
    }

    let consistency_page = get_report
        .execute(ReportQuery {
            report_type: ReportType::DataConsistency,
            filters: ReportFilters::DataConsistency(DataConsistencyReportFilter {
                material_id: None,
                only_inconsistent: true,
            }),
            page: PageQuery {
                page: 1,
                page_size: 20,
            },
        })
        .await?;

    if consistency_page.page != 1 {
        return Err(test_error(
            "data consistency check did not return a valid page",
        ));
    }

    let exported = export
        .execute(ReportExportRequest {
            report_type: ReportType::CurrentStock,
            filters: empty_filters(ReportType::CurrentStock),
            format: ReportExportFormat::Csv,
            include_headers: true,
        })
        .await?;

    if exported.content_type != "text/csv; charset=utf-8"
        || !exported.filename.ends_with(".csv")
        || !exported.body.contains("material_id")
    {
        return Err(test_error("current stock CSV export is incomplete"));
    }

    Ok(())
}

async fn maybe_pool() -> TestResult<Option<PgPool>> {
    if env::var("PHASE9_RUN_DB_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping Phase 9 PostgreSQL reporting tests; set PHASE9_RUN_DB_TESTS=1 to run");
        return Ok(None);
    }

    let Some(database_url) = database_url() else {
        eprintln!("skipping Phase 9 PostgreSQL reporting tests; DATABASE_URL is not set");
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

fn test_state(db_pool: PgPool) -> AppState {
    AppState {
        db_pool,
        jwt_secret: "phase9-test-secret".to_string(),
        jwt_issuer: "ims-phase9-test".to_string(),
        jwt_expires_seconds: 3600,
        jwt_refresh_expires_seconds: 7200,
    }
}

async fn assert_phase9_reporting_schema_contract(pool: &PgPool) -> TestResult<()> {
    let missing_relations: Vec<String> = sqlx::query_scalar(
        r#"
        WITH required(relname, relkind) AS (
            VALUES
                ('rpt_current_stock', 'm'),
                ('rpt_inventory_value', 'm'),
                ('rpt_quality_status', 'm'),
                ('rpt_mrp_shortage', 'm'),
                ('rpt_low_stock_alert', 'm'),
                ('rpt_stock_by_zone', 'm'),
                ('rpt_bin_stock_summary', 'm'),
                ('rpt_batch_stock_summary', 'm'),
                ('rpt_data_consistency_check', 'v')
        )
        SELECT 'rpt.' || required.relname
        FROM required
        WHERE NOT EXISTS (
            SELECT 1
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'rpt'
              AND c.relname = required.relname
              AND c.relkind = required.relkind::"char"
        )
        ORDER BY 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !missing_relations.is_empty() {
        return Err(test_error(format!(
            "Phase 9 reporting schema contract is missing: {}",
            missing_relations.join(", ")
        )));
    }

    let fn_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_proc p
            JOIN pg_namespace n ON n.oid = p.pronamespace
            WHERE n.nspname = 'rpt'
              AND p.proname = 'refresh_all_materialized_views'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !fn_exists {
        return Err(test_error("rpt.refresh_all_materialized_views is missing"));
    }

    Ok(())
}

fn phase9_report_types() -> [ReportType; 9] {
    [
        ReportType::CurrentStock,
        ReportType::InventoryValue,
        ReportType::QualityStatus,
        ReportType::MrpShortage,
        ReportType::LowStockAlert,
        ReportType::StockByZone,
        ReportType::BinStockSummary,
        ReportType::BatchStockSummary,
        ReportType::DataConsistency,
    ]
}

fn empty_filters(report_type: ReportType) -> ReportFilters {
    match report_type {
        ReportType::CurrentStock => ReportFilters::CurrentStock(CurrentStockReportFilter {
            material_id: None,
            material_name: None,
            bin_code: None,
            batch_number: None,
            quality_status: None,
            zone_code: None,
            only_available: false,
        }),
        ReportType::InventoryValue => ReportFilters::InventoryValue(InventoryValueReportFilter {
            material_id: None,
            material_type: None,
            only_positive_value: false,
            sort_by: None,
            sort_order: None,
        }),
        ReportType::QualityStatus => ReportFilters::QualityStatus(QualityStatusReportFilter {
            material_id: None,
            quality_status: None,
            batch_number: None,
        }),
        ReportType::MrpShortage => ReportFilters::MrpShortage(MrpShortageReportFilter {
            run_id: None,
            material_id: None,
            suggestion_type: None,
            only_open: false,
            date_from: None,
            date_to: None,
        }),
        ReportType::LowStockAlert => ReportFilters::LowStockAlert(LowStockAlertReportFilter {
            material_id: None,
            material_type: None,
            severity: None,
        }),
        ReportType::StockByZone => ReportFilters::StockByZone(StockByZoneReportFilter {
            material_id: None,
            material_type: None,
        }),
        ReportType::BinStockSummary => {
            ReportFilters::BinStockSummary(BinStockSummaryReportFilter {
                bin_code: None,
                zone_code: None,
                only_over_capacity: false,
                only_occupied: false,
            })
        }
        ReportType::BatchStockSummary => {
            ReportFilters::BatchStockSummary(BatchStockSummaryReportFilter {
                material_id: None,
                batch_number: None,
                quality_status: None,
                expiry_date_before: None,
                only_expired: false,
                only_expiring: false,
            })
        }
        ReportType::DataConsistency => {
            ReportFilters::DataConsistency(DataConsistencyReportFilter {
                material_id: None,
                only_inconsistent: false,
            })
        }
    }
}

fn test_error(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    Box::new(std::io::Error::other(message.into()))
}
