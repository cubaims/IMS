use std::{env, error::Error, fs, io, sync::Arc};

use cuba_inventory::{
    application::{
        ApproveInventoryCountInput, CancelInventoryCountInput, CloseInventoryCountInput,
        CreateInventoryCountInput, GenerateInventoryCountLinesInput,
        InventoryCountApplicationError, PostInventoryCountInput, SubmitInventoryCountInput,
        UpdateInventoryCountLineInput, inventory_count_service::InventoryCountService,
    },
    domain::{InventoryCountScope, InventoryCountStatus, InventoryCountType},
    infrastructure::PostgresInventoryCountRepository,
};
use rust_decimal::Decimal;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::OffsetDateTime;
use uuid::Uuid;

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

struct Phase7Fixture {
    bin_code: String,
    gain_material: String,
    gain_batch: String,
    loss_material: String,
    loss_batch: String,
}

#[tokio::test]
async fn phase7_repository_lifecycle_repeats_and_rollback_on_real_postgres() -> TestResult<()> {
    let Some(pool) = maybe_pool().await? else {
        return Ok(());
    };

    assert_phase7_schema_contract(&pool).await?;

    let fixture = create_fixture_stock(&pool).await?;
    let service = InventoryCountService::new(Arc::new(PostgresInventoryCountRepository::new(
        pool.clone(),
    )));

    let posted_count_doc_id = assert_lifecycle_and_repeat_guards(&service, &pool, &fixture).await?;
    assert_header_status(&pool, &posted_count_doc_id, "CLOSED").await?;

    assert_posting_rolls_back_when_later_loss_line_fails(&service, &pool, &fixture).await?;

    Ok(())
}

async fn maybe_pool() -> TestResult<Option<PgPool>> {
    if env::var("PHASE7_RUN_DB_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping Phase 7 PostgreSQL tests; set PHASE7_RUN_DB_TESTS=1 to run");
        return Ok(None);
    }

    let Some(database_url) = database_url() else {
        eprintln!("skipping Phase 7 PostgreSQL tests; DATABASE_URL is not set");
        return Ok(None);
    };

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;

    Ok(Some(pool))
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

async fn assert_phase7_schema_contract(pool: &PgPool) -> TestResult<()> {
    let missing: Vec<String> = sqlx::query_scalar(
        r#"
        WITH required(table_name, column_name) AS (
            VALUES
                ('wms_inventory_count_h', 'count_scope'),
                ('wms_inventory_count_h', 'zone_code'),
                ('wms_inventory_count_h', 'bin_code'),
                ('wms_inventory_count_h', 'material_id'),
                ('wms_inventory_count_h', 'batch_number'),
                ('wms_inventory_count_h', 'closed_at'),
                ('wms_inventory_count_h', 'remark'),
                ('wms_inventory_count_d', 'quality_status'),
                ('wms_inventory_count_d', 'counted_qty'),
                ('wms_inventory_count_d', 'difference_qty'),
                ('wms_inventory_count_d', 'difference_reason'),
                ('wms_inventory_count_d', 'transaction_id'),
                ('wms_inventory_count_d', 'status'),
                ('wms_inventory_count_d', 'remark'),
                ('wms_inventory_count_d', 'updated_at')
        )
        SELECT required.table_name || '.' || required.column_name
        FROM required
        WHERE NOT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'wms'
              AND table_name = required.table_name
              AND column_name = required.column_name
        )
        ORDER BY required.table_name, required.column_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !missing.is_empty() {
        return Err(test_error(format!(
            "Phase 7 schema contract is missing columns: {}",
            missing.join(", ")
        )));
    }

    let sequence_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'wms'
              AND c.relname = 'seq_inventory_count_doc'
              AND c.relkind = 'S'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !sequence_exists {
        return Err(test_error("missing wms.seq_inventory_count_doc"));
    }

    Ok(())
}

async fn create_fixture_stock(pool: &PgPool) -> TestResult<Phase7Fixture> {
    let suffix = unique_code("", 10);
    let fixture = Phase7Fixture {
        bin_code: format!("P7B{suffix}"),
        gain_material: format!("P7G{suffix}"),
        gain_batch: format!("B-P7G-{suffix}"),
        loss_material: format!("P7L{suffix}"),
        loss_batch: format!("B-P7L-{suffix}"),
    };

    sqlx::query(
        r#"
        INSERT INTO mdm.mdm_storage_bins (
            bin_code, zone, bin_type, capacity, current_occupied, status, notes
        )
        VALUES ($1, 'RM', 'TEST', 10000, 0, '正常', 'Phase 7 integration test bin')
        ON CONFLICT (bin_code) DO NOTHING
        "#,
    )
    .bind(&fixture.bin_code)
    .execute(pool)
    .await?;

    insert_material(pool, &fixture.gain_material, "Phase 7 gain material").await?;
    insert_material(pool, &fixture.loss_material, "Phase 7 loss material").await?;
    insert_batch(pool, &fixture.gain_batch, &fixture.gain_material).await?;
    insert_batch(pool, &fixture.loss_batch, &fixture.loss_material).await?;

    post_inventory_transaction(
        pool,
        "101",
        &fixture.gain_material,
        10,
        None,
        Some(&fixture.bin_code),
        Some(&fixture.gain_batch),
        "PHASE7-IT-SEED",
        "seed gain fixture",
    )
    .await?;
    post_inventory_transaction(
        pool,
        "101",
        &fixture.loss_material,
        10,
        None,
        Some(&fixture.bin_code),
        Some(&fixture.loss_batch),
        "PHASE7-IT-SEED",
        "seed loss fixture",
    )
    .await?;

    Ok(fixture)
}

async fn insert_material(pool: &PgPool, material_id: &str, material_name: &str) -> TestResult<()> {
    sqlx::query(
        r#"
        INSERT INTO mdm.mdm_materials (
            material_id, material_name, material_type, base_unit, default_zone,
            safety_stock, reorder_point, standard_price, map_price, price_control,
            current_stock, status
        )
        VALUES ($1, $2, '原材料', 'PCS', 'RM', 0, 0, 1, 1, 'Moving Average', 0, '正常')
        ON CONFLICT (material_id) DO NOTHING
        "#,
    )
    .bind(material_id)
    .bind(material_name)
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_batch(pool: &PgPool, batch_number: &str, material_id: &str) -> TestResult<()> {
    sqlx::query(
        r#"
        INSERT INTO wms.wms_batches (
            batch_number, material_id, production_date, expiry_date, quality_grade,
            current_stock, current_bin, quality_status
        )
        VALUES ($1, $2, CURRENT_DATE, CURRENT_DATE + 365, 'A', 0, NULL, '合格')
        ON CONFLICT (batch_number) DO NOTHING
        "#,
    )
    .bind(batch_number)
    .bind(material_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn assert_lifecycle_and_repeat_guards(
    service: &InventoryCountService<PostgresInventoryCountRepository>,
    pool: &PgPool,
    fixture: &Phase7Fixture,
) -> TestResult<String> {
    let created = service
        .create_count(create_count_input(fixture, "lifecycle"))
        .await?;

    let duplicate_scope = service
        .create_count(create_count_input(fixture, "duplicated scope"))
        .await;
    assert!(matches!(
        duplicate_scope,
        Err(InventoryCountApplicationError::DuplicatedScope)
    ));

    let generated = service
        .generate_lines(GenerateInventoryCountLinesInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
        })
        .await?;

    let duplicate_generate = service
        .generate_lines(GenerateInventoryCountLinesInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
        })
        .await;
    assert!(matches!(
        duplicate_generate,
        Err(InventoryCountApplicationError::StatusInvalid)
    ));

    let gain_line_no = count_fixture_lines(service, &generated.lines, fixture).await?;

    let submitted = service
        .submit(SubmitInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
            remark: Some("submit lifecycle count".to_string()),
        })
        .await?;
    assert_eq!(submitted.status, InventoryCountStatus::Submitted);

    let duplicate_submit = service
        .submit(SubmitInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
            remark: Some("repeat submit".to_string()),
        })
        .await;
    assert!(matches!(
        duplicate_submit,
        Err(InventoryCountApplicationError::StatusInvalid)
    ));

    service
        .approve(ApproveInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            approved: true,
            operator: "phase7_it".to_string(),
            remark: Some("approve lifecycle count".to_string()),
        })
        .await?;

    let posted = service
        .post(PostInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            posting_date: OffsetDateTime::now_utc(),
            operator: "phase7_it".to_string(),
            remark: Some("post lifecycle count".to_string()),
        })
        .await?;
    assert_eq!(posted.status, InventoryCountStatus::Posted);

    let mut movement_types = posted
        .transactions
        .iter()
        .map(|transaction| transaction.movement_type.as_str())
        .collect::<Vec<_>>();
    movement_types.sort_unstable();
    assert_eq!(movement_types, vec!["701", "702"]);

    assert_successful_posting_persisted(pool, &created.count_doc_id).await?;

    let duplicate_post = service
        .post(PostInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            posting_date: OffsetDateTime::now_utc(),
            operator: "phase7_it".to_string(),
            remark: Some("repeat post".to_string()),
        })
        .await;
    assert!(matches!(
        duplicate_post,
        Err(InventoryCountApplicationError::StatusInvalid)
    ));

    let cancel_posted = service
        .cancel(CancelInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
            remark: Some("cancel posted count".to_string()),
        })
        .await;
    assert!(matches!(
        cancel_posted,
        Err(InventoryCountApplicationError::AlreadyPosted)
    ));

    let closed = service
        .close(CloseInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
            remark: Some("close lifecycle count".to_string()),
        })
        .await?;
    assert_eq!(closed.status, InventoryCountStatus::Closed);

    let update_closed = service
        .update_line(UpdateInventoryCountLineInput {
            count_doc_id: created.count_doc_id.clone(),
            line_no: gain_line_no,
            counted_qty: Decimal::from(1),
            difference_reason: Some("should fail".to_string()),
            remark: Some("closed count is read only".to_string()),
            operator: "phase7_it".to_string(),
        })
        .await;
    assert!(matches!(
        update_closed,
        Err(InventoryCountApplicationError::StatusInvalid)
    ));

    Ok(created.count_doc_id)
}

async fn assert_posting_rolls_back_when_later_loss_line_fails(
    service: &InventoryCountService<PostgresInventoryCountRepository>,
    pool: &PgPool,
    fixture: &Phase7Fixture,
) -> TestResult<()> {
    let created = service
        .create_count(create_count_input(fixture, "rollback"))
        .await?;
    let generated = service
        .generate_lines(GenerateInventoryCountLinesInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
        })
        .await?;
    count_fixture_lines(service, &generated.lines, fixture).await?;

    service
        .submit(SubmitInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            operator: "phase7_it".to_string(),
            remark: Some("submit rollback count".to_string()),
        })
        .await?;
    service
        .approve(ApproveInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            approved: true,
            operator: "phase7_it".to_string(),
            remark: Some("approve rollback count".to_string()),
        })
        .await?;

    let drained_qty = stock_qty(
        pool,
        &fixture.loss_material,
        &fixture.loss_batch,
        &fixture.bin_code,
    )
    .await?;
    if drained_qty <= 0 {
        return Err(test_error(
            "loss fixture stock should be positive before drain",
        ));
    }

    post_inventory_transaction(
        pool,
        "261",
        &fixture.loss_material,
        drained_qty,
        Some(&fixture.bin_code),
        None,
        Some(&fixture.loss_batch),
        "PHASE7-IT-DRAIN",
        "drain loss fixture before rollback assertion",
    )
    .await?;

    let post_result = service
        .post(PostInventoryCountInput {
            count_doc_id: created.count_doc_id.clone(),
            posting_date: OffsetDateTime::now_utc(),
            operator: "phase7_it".to_string(),
            remark: Some("post rollback count".to_string()),
        })
        .await;

    post_inventory_transaction(
        pool,
        "101",
        &fixture.loss_material,
        drained_qty,
        None,
        Some(&fixture.bin_code),
        Some(&fixture.loss_batch),
        "PHASE7-IT-RESTORE",
        "restore loss fixture after rollback assertion",
    )
    .await?;

    assert!(matches!(
        post_result,
        Err(InventoryCountApplicationError::DifferencePostFailed(_))
    ));

    assert_no_half_posting(pool, &created.count_doc_id).await?;

    let cancelled = service
        .cancel(CancelInventoryCountInput {
            count_doc_id: created.count_doc_id,
            operator: "phase7_it".to_string(),
            remark: Some("cancel rollback count".to_string()),
        })
        .await?;
    assert_eq!(cancelled.status, InventoryCountStatus::Cancelled);

    Ok(())
}

fn create_count_input(fixture: &Phase7Fixture, remark: &str) -> CreateInventoryCountInput {
    CreateInventoryCountInput {
        count_type: InventoryCountType::Cycle,
        count_scope: InventoryCountScope::Bin,
        zone_code: None,
        bin_code: Some(fixture.bin_code.clone()),
        material_id: None,
        batch_number: None,
        operator: "phase7_it".to_string(),
        remark: Some(format!("phase7 integration {remark}")),
    }
}

async fn count_fixture_lines(
    service: &InventoryCountService<PostgresInventoryCountRepository>,
    lines: &[cuba_inventory::domain::InventoryCountLine],
    fixture: &Phase7Fixture,
) -> TestResult<i32> {
    let mut gain_line_no = None;

    for line in lines {
        let counted_qty = if line.material_id == fixture.gain_material {
            gain_line_no = Some(line.line_no);
            line.system_qty + Decimal::from(1)
        } else if line.material_id == fixture.loss_material {
            line.system_qty - Decimal::from(1)
        } else {
            line.system_qty
        };

        let difference_reason = if line.material_id == fixture.gain_material {
            Some("phase7 integration gain".to_string())
        } else if line.material_id == fixture.loss_material {
            Some("phase7 integration loss".to_string())
        } else {
            None
        };

        service
            .update_line(UpdateInventoryCountLineInput {
                count_doc_id: line.count_doc_id.clone(),
                line_no: line.line_no,
                counted_qty,
                difference_reason,
                remark: Some("phase7 integration counted".to_string()),
                operator: "phase7_it".to_string(),
            })
            .await?;
    }

    gain_line_no.ok_or_else(|| test_error("gain fixture line was not generated"))
}

async fn assert_successful_posting_persisted(pool: &PgPool, count_doc_id: &str) -> TestResult<()> {
    assert_header_status(pool, count_doc_id, "POSTED").await?;

    let line_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM wms.wms_inventory_count_d
        WHERE count_doc_id = $1
          AND movement_type::text IN ('701', '702')
          AND transaction_id IS NOT NULL
          AND status = 'POSTED'
        "#,
    )
    .bind(count_doc_id)
    .fetch_one(pool)
    .await?;
    assert_eq!(line_count, 2);

    let transaction_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM wms.wms_transactions
        WHERE reference_doc = $1
          AND movement_type::text IN ('701', '702')
        "#,
    )
    .bind(count_doc_id)
    .fetch_one(pool)
    .await?;
    assert_eq!(transaction_count, 2);

    Ok(())
}

async fn assert_no_half_posting(pool: &PgPool, count_doc_id: &str) -> TestResult<()> {
    assert_header_status(pool, count_doc_id, "APPROVED").await?;

    let transaction_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM wms.wms_transactions
        WHERE reference_doc = $1
          AND movement_type::text IN ('701', '702')
        "#,
    )
    .bind(count_doc_id)
    .fetch_one(pool)
    .await?;
    assert_eq!(transaction_count, 0);

    let half_written_lines: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM wms.wms_inventory_count_d
        WHERE count_doc_id = $1
          AND transaction_id IS NOT NULL
        "#,
    )
    .bind(count_doc_id)
    .fetch_one(pool)
    .await?;
    assert_eq!(half_written_lines, 0);

    Ok(())
}

async fn assert_header_status(pool: &PgPool, count_doc_id: &str, status: &str) -> TestResult<()> {
    let actual: String = sqlx::query_scalar(
        r#"
        SELECT status
        FROM wms.wms_inventory_count_h
        WHERE count_doc_id = $1
        "#,
    )
    .bind(count_doc_id)
    .fetch_one(pool)
    .await?;

    assert_eq!(actual, status);
    Ok(())
}

async fn stock_qty(
    pool: &PgPool,
    material_id: &str,
    batch_number: &str,
    bin_code: &str,
) -> TestResult<i32> {
    let qty: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(qty), 0)::bigint
        FROM wms.wms_bin_stock
        WHERE material_id = $1
          AND batch_number = $2
          AND bin_code = $3
        "#,
    )
    .bind(material_id)
    .bind(batch_number)
    .bind(bin_code)
    .fetch_one(pool)
    .await?;

    i32::try_from(qty).map_err(|_| test_error(format!("stock quantity out of i32 range: {qty}")))
}

async fn post_inventory_transaction(
    pool: &PgPool,
    movement_type: &str,
    material_id: &str,
    quantity: i32,
    from_bin: Option<&str>,
    to_bin: Option<&str>,
    batch_number: Option<&str>,
    reference_doc: &str,
    notes: &str,
) -> TestResult<()> {
    sqlx::query(
        r#"
        SELECT wms.post_inventory_transaction(
            $1,
            $2::wms.movement_type,
            $3,
            $4,
            $5,
            $6,
            $7,
            NULL,
            'phase7_it',
            '合格'::mdm.quality_status,
            $8,
            $9,
            NOW(),
            1
        )
        "#,
    )
    .bind(unique_code("P7TX", 30))
    .bind(movement_type)
    .bind(material_id)
    .bind(quantity)
    .bind(from_bin)
    .bind(to_bin)
    .bind(batch_number)
    .bind(reference_doc)
    .bind(notes)
    .execute(pool)
    .await?;

    Ok(())
}

fn unique_code(prefix: &str, max_len: usize) -> String {
    format!("{prefix}{}", Uuid::now_v7().to_string().replace('-', ""))
        .chars()
        .take(max_len)
        .collect()
}

fn test_error(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    Box::new(io::Error::other(message.into()))
}
