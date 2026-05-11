-- Phase 7 inventory count contract hardening.
--
-- Existing developer databases may already have the older count schema from the
-- original v9 baseline. This migration is intentionally additive and
-- idempotent: it keeps legacy columns, backfills the Phase 7 columns used by
-- the Rust repository, and relaxes old NOT NULL constraints that would block
-- the new command-side flow.

CREATE SEQUENCE IF NOT EXISTS wms.seq_inventory_count_doc START 1;

ALTER TABLE wms.wms_inventory_count_h
    ADD COLUMN IF NOT EXISTS count_scope VARCHAR(20),
    ADD COLUMN IF NOT EXISTS zone_code VARCHAR(10),
    ADD COLUMN IF NOT EXISTS bin_code VARCHAR(20),
    ADD COLUMN IF NOT EXISTS material_id VARCHAR(20),
    ADD COLUMN IF NOT EXISTS batch_number VARCHAR(30),
    ADD COLUMN IF NOT EXISTS closed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS remark TEXT;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_h'
          AND column_name = 'zone'
    ) THEN
        EXECUTE 'UPDATE wms.wms_inventory_count_h SET zone_code = COALESCE(zone_code, zone)';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_h'
          AND column_name = 'notes'
    ) THEN
        EXECUTE 'UPDATE wms.wms_inventory_count_h SET remark = COALESCE(remark, notes)';
    END IF;
END $$;

UPDATE wms.wms_inventory_count_h h
SET
    bin_code = COALESCE(h.bin_code, first_line.bin_code),
    material_id = COALESCE(h.material_id, first_line.material_id),
    batch_number = COALESCE(h.batch_number, first_line.batch_number)
FROM (
    SELECT DISTINCT ON (count_doc_id)
        count_doc_id,
        bin_code,
        material_id,
        batch_number
    FROM wms.wms_inventory_count_d
    ORDER BY count_doc_id, line_no
) first_line
WHERE h.count_doc_id = first_line.count_doc_id;

ALTER TABLE wms.wms_inventory_count_h
    DROP CONSTRAINT IF EXISTS wms_inventory_count_h_count_type_check,
    DROP CONSTRAINT IF EXISTS wms_inventory_count_h_status_check;

UPDATE wms.wms_inventory_count_h
SET count_type = CASE count_type
    WHEN '周期盘点' THEN 'CYCLE'
    WHEN '年度盘点' THEN 'YEAR_END'
    WHEN '抽盘' THEN 'CYCLE'
    WHEN 'REGULAR' THEN 'REGULAR'
    WHEN 'CYCLE' THEN 'CYCLE'
    WHEN 'ADJUSTMENT' THEN 'ADJUSTMENT'
    WHEN 'YEAR_END' THEN 'YEAR_END'
    ELSE 'REGULAR'
END;

UPDATE wms.wms_inventory_count_h
SET status = CASE status
    WHEN '草稿' THEN 'DRAFT'
    WHEN '盘点中' THEN 'COUNTING'
    WHEN '待审批' THEN 'SUBMITTED'
    WHEN '已过账' THEN 'POSTED'
    WHEN '取消' THEN 'CANCELLED'
    WHEN 'DRAFT' THEN 'DRAFT'
    WHEN 'COUNTING' THEN 'COUNTING'
    WHEN 'SUBMITTED' THEN 'SUBMITTED'
    WHEN 'APPROVED' THEN 'APPROVED'
    WHEN 'POSTED' THEN 'POSTED'
    WHEN 'CLOSED' THEN 'CLOSED'
    WHEN 'CANCELLED' THEN 'CANCELLED'
    ELSE 'DRAFT'
END;

UPDATE wms.wms_inventory_count_h
SET count_scope = COALESCE(
    count_scope,
    CASE
        WHEN bin_code IS NOT NULL THEN 'BIN'
        WHEN material_id IS NOT NULL THEN 'MATERIAL'
        WHEN zone_code IS NOT NULL THEN 'ZONE'
        ELSE 'BIN'
    END
);

ALTER TABLE wms.wms_inventory_count_h
    ALTER COLUMN count_type SET DEFAULT 'REGULAR',
    ALTER COLUMN count_scope SET DEFAULT 'BIN',
    ALTER COLUMN status SET DEFAULT 'DRAFT';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'wms_inventory_count_h_count_type_check'
          AND conrelid = 'wms.wms_inventory_count_h'::regclass
    ) THEN
        ALTER TABLE wms.wms_inventory_count_h
            ADD CONSTRAINT wms_inventory_count_h_count_type_check
            CHECK (count_type IN ('REGULAR', 'CYCLE', 'ADJUSTMENT', 'YEAR_END'));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'wms_inventory_count_h_count_scope_check'
          AND conrelid = 'wms.wms_inventory_count_h'::regclass
    ) THEN
        ALTER TABLE wms.wms_inventory_count_h
            ADD CONSTRAINT wms_inventory_count_h_count_scope_check
            CHECK (count_scope IN ('FULL', 'ZONE', 'BIN', 'MATERIAL', 'BATCH', 'CYCLE'));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'wms_inventory_count_h_status_check'
          AND conrelid = 'wms.wms_inventory_count_h'::regclass
    ) THEN
        ALTER TABLE wms.wms_inventory_count_h
            ADD CONSTRAINT wms_inventory_count_h_status_check
            CHECK (status IN ('DRAFT', 'COUNTING', 'SUBMITTED', 'APPROVED', 'POSTED', 'CLOSED', 'CANCELLED'));
    END IF;
END $$;

ALTER TABLE wms.wms_inventory_count_d
    ADD COLUMN IF NOT EXISTS quality_status mdm.quality_status,
    ADD COLUMN IF NOT EXISTS counted_qty INTEGER,
    ADD COLUMN IF NOT EXISTS difference_qty INTEGER,
    ADD COLUMN IF NOT EXISTS difference_reason TEXT,
    ADD COLUMN IF NOT EXISTS transaction_id VARCHAR(30),
    ADD COLUMN IF NOT EXISTS status VARCHAR(20),
    ADD COLUMN IF NOT EXISTS remark TEXT,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_d'
          AND column_name = 'physical_qty'
    ) THEN
        EXECUTE 'ALTER TABLE wms.wms_inventory_count_d ALTER COLUMN physical_qty DROP NOT NULL';
        EXECUTE 'UPDATE wms.wms_inventory_count_d SET counted_qty = COALESCE(counted_qty, physical_qty)';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_d'
          AND column_name = 'variance_qty'
    ) THEN
        EXECUTE 'UPDATE wms.wms_inventory_count_d SET difference_qty = COALESCE(difference_qty, variance_qty)';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_d'
          AND column_name = 'variance_reason'
    ) THEN
        EXECUTE 'UPDATE wms.wms_inventory_count_d SET difference_reason = COALESCE(difference_reason, variance_reason)';
        EXECUTE 'UPDATE wms.wms_inventory_count_d SET remark = COALESCE(remark, variance_reason)';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'wms'
          AND table_name = 'wms_inventory_count_d'
          AND column_name = 'adjustment_transaction_id'
    ) THEN
        EXECUTE 'UPDATE wms.wms_inventory_count_d SET transaction_id = COALESCE(transaction_id, adjustment_transaction_id)';
    END IF;
END $$;

UPDATE wms.wms_inventory_count_d d
SET quality_status = COALESCE(d.quality_status, b.quality_status, '合格'::mdm.quality_status)
FROM wms.wms_batches b
WHERE d.batch_number = b.batch_number
  AND d.material_id = b.material_id;

UPDATE wms.wms_inventory_count_d
SET
    quality_status = COALESCE(quality_status, '合格'::mdm.quality_status),
    status = COALESCE(
        status,
        CASE
            WHEN adjusted IS TRUE OR transaction_id IS NOT NULL THEN 'POSTED'
            WHEN counted_qty IS NOT NULL THEN 'COUNTED'
            ELSE 'PENDING'
        END
    );

ALTER TABLE wms.wms_inventory_count_d
    ALTER COLUMN status SET DEFAULT 'PENDING';

ALTER TABLE wms.wms_inventory_count_d
    DROP CONSTRAINT IF EXISTS wms_inventory_count_d_counted_qty_check,
    DROP CONSTRAINT IF EXISTS wms_inventory_count_d_status_check;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'wms_inventory_count_d_counted_qty_check'
          AND conrelid = 'wms.wms_inventory_count_d'::regclass
    ) THEN
        ALTER TABLE wms.wms_inventory_count_d
            ADD CONSTRAINT wms_inventory_count_d_counted_qty_check
            CHECK (counted_qty IS NULL OR counted_qty >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'wms_inventory_count_d_status_check'
          AND conrelid = 'wms.wms_inventory_count_d'::regclass
    ) THEN
        ALTER TABLE wms.wms_inventory_count_d
            ADD CONSTRAINT wms_inventory_count_d_status_check
            CHECK (status IN ('PENDING', 'COUNTED', 'POSTED'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_count_h_scope
    ON wms.wms_inventory_count_h(count_scope, status);

CREATE INDEX IF NOT EXISTS idx_count_d_status
    ON wms.wms_inventory_count_d(count_doc_id, status);
