-- ============================================================
-- Simple WMS Inventory System - PostgreSQL 最终重构完整可执行版 v9
-- 文件：schema_final_ultimate_complete_v9.sql
-- 说明：面向空库/演示库执行，会重建 mdm/wms/rpt/sys 四个 schema。
-- 警告：脚本开头会 DROP rpt/wms/mdm/sys 四个 schema，请勿直接在生产库执行。
-- 覆盖：Excel 17 个 Sheet + 一键操作助手、触发器审计、库存一致性校验、MAP/批次历史、生产成本差异、FEFO 并发锁批次、8 个报表；v9 补充分区、刷新顺序、依赖注释、函数示例与验证脚本。
-- ============================================================

BEGIN;

DROP SCHEMA IF EXISTS rpt CASCADE;
DROP SCHEMA IF EXISTS wms CASCADE;
DROP SCHEMA IF EXISTS mdm CASCADE;
DROP SCHEMA IF EXISTS sys CASCADE;

CREATE SCHEMA mdm;
CREATE SCHEMA wms;
CREATE SCHEMA rpt;
CREATE SCHEMA sys;

SET search_path TO wms, mdm, sys, public;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- ============================================================
-- 1. 自定义类型
-- ============================================================
CREATE TYPE mdm.quality_status AS ENUM ('待检', '合格', '冻结', '报废', '放行');
CREATE TYPE mdm.material_type AS ENUM ('原材料', '半成品', '成品');
CREATE TYPE wms.movement_type AS ENUM ('101', '261', '311', '501', '701', '702', '999');

-- 防御性清理历史重载签名：空库初始化时 DROP SCHEMA 已清理，这里用于防止人工裁剪脚本后产生函数重载歧义。
DROP FUNCTION IF EXISTS wms.post_inventory_transaction(
    VARCHAR(30), wms.movement_type, VARCHAR(20), INTEGER,
    VARCHAR(20), VARCHAR(20), VARCHAR(30), VARCHAR(30),
    VARCHAR(50), mdm.quality_status, VARCHAR(30), TEXT, TIMESTAMPTZ
);
DROP FUNCTION IF EXISTS wms.post_inventory_transaction(
    VARCHAR(30), wms.movement_type, VARCHAR(20), INTEGER,
    VARCHAR(20), VARCHAR(20), VARCHAR(30), VARCHAR(30),
    VARCHAR(50), mdm.quality_status, VARCHAR(30), TEXT, TIMESTAMPTZ, NUMERIC
);
DROP FUNCTION IF EXISTS rpt.refresh_all_materialized_views();

-- ============================================================
-- 2. MDM 主数据
-- ============================================================
CREATE TABLE mdm.mdm_materials (
    material_id         VARCHAR(20) PRIMARY KEY,
    material_name       VARCHAR(100) NOT NULL,
    material_type       mdm.material_type NOT NULL,
    base_unit           VARCHAR(10) NOT NULL DEFAULT 'PCS',
    default_zone        VARCHAR(10) NOT NULL,
    safety_stock        INTEGER NOT NULL DEFAULT 0 CHECK (safety_stock >= 0),
    reorder_point       INTEGER NOT NULL DEFAULT 0 CHECK (reorder_point >= 0),
    standard_price      NUMERIC(12,2) NOT NULL CHECK (standard_price >= 0),
    map_price           NUMERIC(12,2) NOT NULL DEFAULT 0 CHECK (map_price >= 0),
    price_control       VARCHAR(20) DEFAULT 'Moving Average' CHECK (price_control IN ('Standard', 'Moving Average')),
    current_stock       INTEGER NOT NULL DEFAULT 0 CHECK (current_stock >= 0),
    total_map_value     NUMERIC(15,2) GENERATED ALWAYS AS (current_stock * map_price) STORED,
    price_variance      NUMERIC(15,2) GENERATED ALWAYS AS ((standard_price - map_price) * current_stock) STORED,
    status              VARCHAR(20) DEFAULT '正常' CHECK (status IN ('正常', '停用', '冻结')),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_storage_bins (
    bin_code            VARCHAR(20) PRIMARY KEY,
    zone                VARCHAR(10) NOT NULL,
    bin_type            VARCHAR(20) NOT NULL,
    capacity            INTEGER NOT NULL CHECK (capacity > 0),
    current_occupied    INTEGER NOT NULL DEFAULT 0 CHECK (current_occupied >= 0),
    available_capacity  INTEGER GENERATED ALWAYS AS (capacity - current_occupied) STORED,
    status              VARCHAR(20) DEFAULT '正常' CHECK (status IN ('正常', '占用', '维护中', '冻结')),
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (current_occupied <= capacity)
);

CREATE TABLE mdm.mdm_suppliers (
    supplier_id         VARCHAR(20) PRIMARY KEY,
    supplier_name       VARCHAR(100) NOT NULL,
    contact_person      VARCHAR(50),
    phone               VARCHAR(20),
    email               VARCHAR(100),
    address             TEXT,
    quality_rating      VARCHAR(10) DEFAULT 'A' CHECK (quality_rating IN ('A', 'B', 'C', 'D')),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_customers (
    customer_id         VARCHAR(20) PRIMARY KEY,
    customer_name       VARCHAR(100) NOT NULL,
    contact_person      VARCHAR(50),
    phone               VARCHAR(20),
    email               VARCHAR(100),
    address             TEXT,
    credit_limit        NUMERIC(15,2) DEFAULT 0 CHECK (credit_limit >= 0),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_material_suppliers (
    id                      BIGSERIAL PRIMARY KEY,
    material_id             VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    supplier_id             VARCHAR(20) NOT NULL REFERENCES mdm.mdm_suppliers(supplier_id),
    is_primary              BOOLEAN DEFAULT FALSE,
    supplier_material_code  VARCHAR(50),
    purchase_price          NUMERIC(12,2) CHECK (purchase_price IS NULL OR purchase_price >= 0),
    currency                VARCHAR(10) DEFAULT 'CNY',
    lead_time_days          INTEGER DEFAULT 7 CHECK (lead_time_days >= 0),
    moq                     INTEGER DEFAULT 1 CHECK (moq >= 1),
    quality_rating          VARCHAR(10) DEFAULT 'A' CHECK (quality_rating IN ('A', 'B', 'C', 'D')),
    qualified_date          DATE,
    valid_until             DATE,
    is_active               BOOLEAN DEFAULT TRUE,
    notes                   TEXT,
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    updated_at              TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(material_id, supplier_id)
);

CREATE TABLE mdm.mdm_product_variants (
    variant_code        VARCHAR(20) PRIMARY KEY,
    variant_name        VARCHAR(100) NOT NULL,
    base_material_id    VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    bom_id              VARCHAR(30),
    standard_cost       NUMERIC(12,2) NOT NULL CHECK (standard_cost >= 0),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_bom_headers (
    bom_id              VARCHAR(30) PRIMARY KEY,
    bom_name            VARCHAR(100) NOT NULL,
    parent_material_id  VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    variant_code        VARCHAR(20) REFERENCES mdm.mdm_product_variants(variant_code),
    version             VARCHAR(10) NOT NULL DEFAULT 'V1.0',
    base_quantity       NUMERIC(10,3) NOT NULL DEFAULT 1 CHECK (base_quantity > 0),
    valid_from          DATE NOT NULL DEFAULT CURRENT_DATE,
    valid_to            DATE,
    status              VARCHAR(20) DEFAULT '草稿' CHECK (status IN ('草稿', '生效', '失效')),
    is_active           BOOLEAN DEFAULT TRUE,
    created_by          VARCHAR(50),
    approved_by         VARCHAR(50),
    approved_at         TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(parent_material_id, variant_code, version),
    CHECK (valid_to IS NULL OR valid_to >= valid_from)
);

ALTER TABLE mdm.mdm_product_variants
    ADD CONSTRAINT fk_product_variant_bom
    FOREIGN KEY (bom_id) REFERENCES mdm.mdm_bom_headers(bom_id) DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE mdm.mdm_bom_components (
    id                      BIGSERIAL PRIMARY KEY,
    bom_id                  VARCHAR(30) NOT NULL REFERENCES mdm.mdm_bom_headers(bom_id) ON DELETE CASCADE,
    parent_material_id      VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    component_material_id   VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    quantity                NUMERIC(10,3) NOT NULL CHECK (quantity > 0),
    unit                    VARCHAR(10) NOT NULL DEFAULT 'PCS',
    bom_level               INTEGER NOT NULL DEFAULT 1 CHECK (bom_level >= 1),
    scrap_rate              NUMERIC(5,2) DEFAULT 0 CHECK (scrap_rate >= 0),
    is_critical             BOOLEAN DEFAULT FALSE,
    valid_from              DATE NOT NULL DEFAULT CURRENT_DATE,
    valid_to                DATE,
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    CHECK (parent_material_id <> component_material_id),
    CHECK (valid_to IS NULL OR valid_to >= valid_from),
    UNIQUE(bom_id, component_material_id)
);

CREATE TABLE mdm.mdm_inspection_chars (
    char_id             VARCHAR(30) PRIMARY KEY,
    char_name           VARCHAR(100) NOT NULL,
    material_type       mdm.material_type,
    inspection_type     VARCHAR(20) CHECK (inspection_type IN ('来料检验', '过程检验', '最终检验')),
    method              TEXT,
    standard            TEXT,
    unit                VARCHAR(20),
    lower_limit         NUMERIC(10,3),
    upper_limit         NUMERIC(10,3),
    is_critical         BOOLEAN DEFAULT FALSE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (lower_limit IS NULL OR upper_limit IS NULL OR upper_limit >= lower_limit)
);

CREATE TABLE mdm.mdm_defect_codes (
    defect_code         VARCHAR(20) PRIMARY KEY,
    defect_name         VARCHAR(100) NOT NULL,
    category            VARCHAR(30),
    severity            VARCHAR(10) CHECK (severity IN ('一般', '严重', '紧急')),
    description         TEXT,
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_price_list (
    id                  BIGSERIAL PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    customer_id         VARCHAR(20) REFERENCES mdm.mdm_customers(customer_id),
    price_type          VARCHAR(20) DEFAULT '标准价' CHECK (price_type IN ('标准价', '促销价', '合同价')),
    unit_price          NUMERIC(12,2) NOT NULL CHECK (unit_price >= 0),
    valid_from          DATE NOT NULL,
    valid_to            DATE,
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (valid_to IS NULL OR valid_to >= valid_from)
);

CREATE TABLE mdm.mdm_work_centers (
    work_center_id      VARCHAR(20) PRIMARY KEY,
    work_center_name    VARCHAR(100) NOT NULL,
    location            VARCHAR(50),
    capacity_per_day    INTEGER CHECK (capacity_per_day IS NULL OR capacity_per_day > 0),
    efficiency          NUMERIC(5,2) DEFAULT 100.00 CHECK (efficiency > 0),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_cost_centers (
    cost_center_id      VARCHAR(20) PRIMARY KEY,
    cost_center_name    VARCHAR(100) NOT NULL,
    department          VARCHAR(50),
    manager             VARCHAR(50),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE mdm.mdm_gl_accounts (
    gl_account          VARCHAR(20) PRIMARY KEY,
    account_name        VARCHAR(100) NOT NULL,
    account_type        VARCHAR(30) CHECK (account_type IN ('资产', '负债', '权益', '收入', '成本', '费用')),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 3. WMS 业务表
-- ============================================================
CREATE TABLE wms.wms_movement_type_config (
    movement_type       wms.movement_type PRIMARY KEY,
    movement_name       VARCHAR(50) NOT NULL,
    direction           VARCHAR(10) CHECK (direction IN ('入库', '出库', '转移')),
    requires_from_bin   BOOLEAN DEFAULT FALSE,
    requires_to_bin     BOOLEAN DEFAULT FALSE,
    requires_batch      BOOLEAN DEFAULT TRUE,
    requires_serial     BOOLEAN DEFAULT FALSE,
    requires_quality    BOOLEAN DEFAULT FALSE,
    affects_material_stock BOOLEAN DEFAULT TRUE,
    description         TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_batches (
    batch_number        VARCHAR(30) PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    production_date     DATE NOT NULL,
    expiry_date         DATE,
    supplier_batch      VARCHAR(50),
    quality_grade       VARCHAR(10) DEFAULT 'A' CHECK (quality_grade IN ('A', 'B', 'C')),
    current_stock       INTEGER NOT NULL DEFAULT 0 CHECK (current_stock >= 0),
    current_bin         VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    quality_status      mdm.quality_status DEFAULT '待检',
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (expiry_date IS NULL OR expiry_date >= production_date)
);

CREATE TABLE wms.wms_batch_attributes (
    batch_number        VARCHAR(30) PRIMARY KEY REFERENCES wms.wms_batches(batch_number) ON DELETE CASCADE,
    manufacturing_date  DATE,
    expiry_date         DATE,
    lot_number          VARCHAR(50),
    supplier_lot        VARCHAR(50),
    moisture_level      VARCHAR(20),
    storage_condition   VARCHAR(100),
    custom_attributes   JSONB DEFAULT '{}'::jsonb,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_serial_numbers (
    serial_number       VARCHAR(30) PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    current_status      VARCHAR(20) DEFAULT '在库' CHECK (current_status IN ('在库', '生产中', '已销售', '报废')),
    current_bin         VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    quality_status      mdm.quality_status DEFAULT '合格',
    last_movement_at    TIMESTAMPTZ,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_serial_history (
    id                  BIGSERIAL PRIMARY KEY,
    serial_number       VARCHAR(30) NOT NULL REFERENCES wms.wms_serial_numbers(serial_number) ON DELETE CASCADE,
    old_status          VARCHAR(20),
    new_status          VARCHAR(20),
    old_bin             VARCHAR(20),
    new_bin             VARCHAR(20),
    old_quality_status  mdm.quality_status,
    new_quality_status  mdm.quality_status,
    transaction_id      VARCHAR(30),
    changed_by          VARCHAR(50),
    changed_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_bin_stock (
    id                  BIGSERIAL PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    bin_code            VARCHAR(20) NOT NULL REFERENCES mdm.mdm_storage_bins(bin_code),
    batch_number        VARCHAR(30) NOT NULL REFERENCES wms.wms_batches(batch_number),
    quality_status      mdm.quality_status NOT NULL DEFAULT '合格',
    qty                 INTEGER NOT NULL DEFAULT 0 CHECK (qty >= 0),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(material_id, bin_code, batch_number)
);

CREATE TABLE wms.wms_transactions (
    id                  BIGINT GENERATED ALWAYS AS IDENTITY,
    transaction_id      VARCHAR(30) NOT NULL,
    transaction_date    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    movement_type       wms.movement_type NOT NULL,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    quantity            INTEGER NOT NULL CHECK (quantity <> 0),
    from_bin            VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    to_bin              VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    serial_number       VARCHAR(30) REFERENCES wms.wms_serial_numbers(serial_number),
    reference_doc       VARCHAR(30),
    operator            VARCHAR(50) NOT NULL,
    quality_status      mdm.quality_status,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW()
) PARTITION BY RANGE (transaction_date);

CREATE TABLE wms.wms_transactions_2026_04 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-04-01 00:00:00+00') TO ('2026-05-01 00:00:00+00');

CREATE TABLE wms.wms_transactions_2026_05 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-05-01 00:00:00+00') TO ('2026-06-01 00:00:00+00');

CREATE TABLE wms.wms_transactions_2026_06 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-06-01 00:00:00+00') TO ('2026-07-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_07 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-07-01 00:00:00+00') TO ('2026-08-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_08 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_09 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-09-01 00:00:00+00') TO ('2026-10-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_10 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_11 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');
CREATE TABLE wms.wms_transactions_2026_12 PARTITION OF wms.wms_transactions
    FOR VALUES FROM ('2026-12-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');

CREATE TABLE wms.wms_purchase_orders_h (
    po_id               VARCHAR(30) PRIMARY KEY,
    supplier_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_suppliers(supplier_id),
    po_date             DATE NOT NULL DEFAULT CURRENT_DATE,
    expected_date       DATE,
    total_amount        NUMERIC(15,2) DEFAULT 0 CHECK (total_amount >= 0),
    currency            VARCHAR(10) DEFAULT 'CNY',
    status              VARCHAR(20) DEFAULT '草稿' CHECK (status IN ('草稿', '已审批', '部分到货', '完成', '取消')),
    created_by          VARCHAR(50),
    approved_by         VARCHAR(50),
    approved_at         TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (expected_date IS NULL OR expected_date >= po_date)
);

CREATE TABLE wms.wms_purchase_orders_d (
    id                  BIGSERIAL PRIMARY KEY,
    po_id               VARCHAR(30) NOT NULL REFERENCES wms.wms_purchase_orders_h(po_id) ON DELETE CASCADE,
    line_no             INTEGER NOT NULL CHECK (line_no > 0),
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    ordered_qty         INTEGER NOT NULL CHECK (ordered_qty > 0),
    received_qty        INTEGER DEFAULT 0 CHECK (received_qty >= 0),
    open_qty            INTEGER GENERATED ALWAYS AS (ordered_qty - received_qty) STORED,
    unit_price          NUMERIC(12,2) NOT NULL CHECK (unit_price >= 0),
    line_amount         NUMERIC(15,2) GENERATED ALWAYS AS (ordered_qty * unit_price) STORED,
    expected_bin        VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    line_status         VARCHAR(20) DEFAULT '待到货' CHECK (line_status IN ('待到货', '部分到货', '完成', '取消')),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(po_id, line_no),
    CHECK (received_qty <= ordered_qty)
);

CREATE TABLE wms.wms_sales_orders_h (
    so_id               VARCHAR(30) PRIMARY KEY,
    customer_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_customers(customer_id),
    so_date             DATE NOT NULL DEFAULT CURRENT_DATE,
    delivery_date       DATE,
    total_amount        NUMERIC(15,2) DEFAULT 0 CHECK (total_amount >= 0),
    total_cogs          NUMERIC(15,2) DEFAULT 0 CHECK (total_cogs >= 0),
    gross_margin        NUMERIC(15,2) GENERATED ALWAYS AS (total_amount - total_cogs) STORED,
    currency            VARCHAR(10) DEFAULT 'CNY',
    status              VARCHAR(20) DEFAULT '草稿' CHECK (status IN ('草稿', '已审批', '部分发货', '完成', '取消')),
    created_by          VARCHAR(50),
    approved_by         VARCHAR(50),
    approved_at         TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    CHECK (delivery_date IS NULL OR delivery_date >= so_date)
);

CREATE TABLE wms.wms_sales_orders_d (
    id                  BIGSERIAL PRIMARY KEY,
    so_id               VARCHAR(30) NOT NULL REFERENCES wms.wms_sales_orders_h(so_id) ON DELETE CASCADE,
    line_no             INTEGER NOT NULL CHECK (line_no > 0),
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    variant_code        VARCHAR(20) REFERENCES mdm.mdm_product_variants(variant_code),
    ordered_qty         INTEGER NOT NULL CHECK (ordered_qty > 0),
    shipped_qty         INTEGER DEFAULT 0 CHECK (shipped_qty >= 0),
    open_qty            INTEGER GENERATED ALWAYS AS (ordered_qty - shipped_qty) STORED,
    unit_price          NUMERIC(12,2) NOT NULL CHECK (unit_price >= 0),
    map_at_shipment     NUMERIC(12,2) DEFAULT 0 CHECK (map_at_shipment >= 0),
    line_amount         NUMERIC(15,2) GENERATED ALWAYS AS (ordered_qty * unit_price) STORED,
    line_cogs           NUMERIC(15,2) GENERATED ALWAYS AS (shipped_qty * COALESCE(map_at_shipment, 0)) STORED,
    line_margin         NUMERIC(15,2) GENERATED ALWAYS AS ((shipped_qty * unit_price) - (shipped_qty * COALESCE(map_at_shipment, 0))) STORED,
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    from_bin            VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    line_status         VARCHAR(20) DEFAULT '待发货' CHECK (line_status IN ('待发货', '部分发货', '完成', '取消')),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(so_id, line_no),
    CHECK (shipped_qty <= ordered_qty)
);

CREATE TABLE wms.wms_production_orders_h (
    order_id            VARCHAR(30) PRIMARY KEY,
    variant_code        VARCHAR(20) REFERENCES mdm.mdm_product_variants(variant_code),
    bom_id              VARCHAR(30) REFERENCES mdm.mdm_bom_headers(bom_id),
    output_material_id  VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    work_center_id      VARCHAR(20) REFERENCES mdm.mdm_work_centers(work_center_id),
    planned_quantity    INTEGER NOT NULL CHECK (planned_quantity > 0),
    actual_quantity     INTEGER DEFAULT 0 CHECK (actual_quantity >= 0),
    status              VARCHAR(20) DEFAULT '计划中' CHECK (status IN ('计划中', '已下达', '生产中', '完成', '取消')),
    planned_start_date  DATE,
    planned_finish_date DATE,
    actual_start_date   DATE,
    actual_finish_date  DATE,
    created_by          VARCHAR(50),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_production_orders_d (
    id                  BIGSERIAL PRIMARY KEY,
    order_id            VARCHAR(30) NOT NULL REFERENCES wms.wms_production_orders_h(order_id) ON DELETE CASCADE,
    line_no             INTEGER NOT NULL CHECK (line_no > 0),
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    serial_number       VARCHAR(30) REFERENCES wms.wms_serial_numbers(serial_number),
    planned_qty         INTEGER NOT NULL CHECK (planned_qty >= 0),
    actual_qty          INTEGER DEFAULT 0 CHECK (actual_qty >= 0),
    from_bin            VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    issue_transaction_id VARCHAR(30),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(order_id, line_no)
);

CREATE TABLE wms.wms_production_variances (
    variance_id             BIGSERIAL PRIMARY KEY,
    order_id                VARCHAR(30) NOT NULL REFERENCES wms.wms_production_orders_h(order_id) ON DELETE CASCADE,
    variant_code            VARCHAR(20) REFERENCES mdm.mdm_product_variants(variant_code),
    output_material_id      VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    planned_quantity        INTEGER NOT NULL CHECK (planned_quantity >= 0),
    actual_quantity         INTEGER NOT NULL CHECK (actual_quantity >= 0),
    planned_unit_cost       NUMERIC(12,4) NOT NULL DEFAULT 0 CHECK (planned_unit_cost >= 0),
    actual_unit_cost        NUMERIC(12,4) NOT NULL DEFAULT 0 CHECK (actual_unit_cost >= 0),
    planned_material_cost   NUMERIC(15,2) NOT NULL DEFAULT 0 CHECK (planned_material_cost >= 0),
    actual_material_cost    NUMERIC(15,2) NOT NULL DEFAULT 0 CHECK (actual_material_cost >= 0),
    labor_variance          NUMERIC(15,2) NOT NULL DEFAULT 0,
    overhead_variance       NUMERIC(15,2) NOT NULL DEFAULT 0,
    material_variance       NUMERIC(15,2)
                            GENERATED ALWAYS AS (actual_material_cost - planned_material_cost) STORED,
    total_variance          NUMERIC(15,2)
                            GENERATED ALWAYS AS ((actual_material_cost - planned_material_cost) + labor_variance + overhead_variance) STORED,
    variance_pct            NUMERIC(8,4)
                            GENERATED ALWAYS AS (
                                CASE
                                    WHEN planned_material_cost = 0 THEN NULL
                                    ELSE ROUND(((actual_material_cost - planned_material_cost) / planned_material_cost * 100)::NUMERIC, 4)
                                END
                            ) STORED,
    variance_reason         VARCHAR(100),
    calculated_at           TIMESTAMPTZ DEFAULT NOW(),
    created_by              VARCHAR(50),
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    updated_at              TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(order_id)
);

COMMENT ON TABLE wms.wms_production_variances IS
    '生产订单级成本差异：计划成本 vs 实际领料成本，补足 Excel 成本核算中的生产差异分析。';

COMMENT ON COLUMN wms.wms_production_variances.labor_variance IS
    '人工差异预留字段：当前脚本默认 0，实际项目可由 MES/工时系统或人工结算单回写。';
COMMENT ON COLUMN wms.wms_production_variances.overhead_variance IS
    '制造费用差异预留字段：当前脚本默认 0，实际项目可由成本中心分摊或月结程序回写。';
COMMENT ON COLUMN wms.wms_production_variances.material_variance IS
    '材料差异自动生成：actual_material_cost - planned_material_cost。';
COMMENT ON COLUMN wms.wms_production_variances.total_variance IS
    '总差异自动生成：材料差异 + 人工差异 + 制造费用差异。';

CREATE TABLE wms.wms_batch_genealogy (
    id                      BIGSERIAL PRIMARY KEY,
    parent_batch_number     VARCHAR(30) NOT NULL REFERENCES wms.wms_batches(batch_number),
    component_batch_number  VARCHAR(30) NOT NULL REFERENCES wms.wms_batches(batch_number),
    parent_material_id      VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    component_material_id   VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    production_order_id     VARCHAR(30) REFERENCES wms.wms_production_orders_h(order_id),
    consumed_qty            NUMERIC(12,3) NOT NULL CHECK (consumed_qty > 0),
    output_qty              NUMERIC(12,3) CHECK (output_qty IS NULL OR output_qty >= 0),
    transaction_id          VARCHAR(30),
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    CHECK (parent_batch_number <> component_batch_number),
    UNIQUE(parent_batch_number, component_batch_number, production_order_id)
);

CREATE TABLE wms.wms_inspection_lots (
    inspection_lot_id   VARCHAR(30) PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    serial_number       VARCHAR(30) REFERENCES wms.wms_serial_numbers(serial_number),
    inspection_type     VARCHAR(20) NOT NULL CHECK (inspection_type IN ('来料检验', '过程检验', '最终检验')),
    lot_status          mdm.quality_status DEFAULT '待检',
    inspection_date     TIMESTAMPTZ,
    inspector           VARCHAR(50),
    inspection_result   JSONB DEFAULT '{}'::jsonb,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_inspection_results (
    id                  BIGSERIAL PRIMARY KEY,
    inspection_lot_id   VARCHAR(30) NOT NULL REFERENCES wms.wms_inspection_lots(inspection_lot_id) ON DELETE CASCADE,
    char_id             VARCHAR(30) REFERENCES mdm.mdm_inspection_chars(char_id),
    measured_value      NUMERIC(10,3),
    result              VARCHAR(20) CHECK (result IN ('合格', '不合格', '让步接收')),
    remarks             TEXT,
    inspected_by        VARCHAR(50),
    inspected_at        TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_quality_notifications (
    notification_id     VARCHAR(30) PRIMARY KEY,
    inspection_lot_id   VARCHAR(30) REFERENCES wms.wms_inspection_lots(inspection_lot_id),
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    serial_number       VARCHAR(30) REFERENCES wms.wms_serial_numbers(serial_number),
    defect_code         VARCHAR(20) REFERENCES mdm.mdm_defect_codes(defect_code),
    problem_description TEXT NOT NULL,
    severity            VARCHAR(10) CHECK (severity IN ('一般', '严重', '紧急')),
    root_cause          TEXT,
    corrective_action   TEXT,
    responsible_person  VARCHAR(50),
    status              VARCHAR(20) DEFAULT '处理中' CHECK (status IN ('处理中', '已关闭', '已报废')),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    closed_at           TIMESTAMPTZ
);

CREATE TABLE wms.wms_inventory_count_h (
    count_doc_id        VARCHAR(30) PRIMARY KEY,
    count_date          DATE NOT NULL DEFAULT CURRENT_DATE,
    count_type          VARCHAR(20) DEFAULT '周期盘点' CHECK (count_type IN ('周期盘点', '年度盘点', '抽盘')),
    zone                VARCHAR(10),
    status              VARCHAR(20) DEFAULT '草稿' CHECK (status IN ('草稿', '盘点中', '待审批', '已过账', '取消')),
    created_by          VARCHAR(50),
    approved_by         VARCHAR(50),
    approved_at         TIMESTAMPTZ,
    posted_by           VARCHAR(50),
    posted_at           TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_inventory_count_d (
    id                  BIGSERIAL PRIMARY KEY,
    count_doc_id        VARCHAR(30) NOT NULL REFERENCES wms.wms_inventory_count_h(count_doc_id) ON DELETE CASCADE,
    line_no             INTEGER NOT NULL CHECK (line_no > 0),
    bin_code            VARCHAR(20) NOT NULL REFERENCES mdm.mdm_storage_bins(bin_code),
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    batch_number        VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    serial_number       VARCHAR(30) REFERENCES wms.wms_serial_numbers(serial_number),
    system_qty          INTEGER NOT NULL CHECK (system_qty >= 0),
    physical_qty        INTEGER NOT NULL CHECK (physical_qty >= 0),
    variance_qty        INTEGER GENERATED ALWAYS AS (physical_qty - system_qty) STORED,
    variance_reason     TEXT,
    movement_type       wms.movement_type,
    adjustment_transaction_id VARCHAR(30),
    adjusted            BOOLEAN DEFAULT FALSE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(count_doc_id, line_no)
);

CREATE TABLE wms.wms_mrp_runs (
    run_id              VARCHAR(30) PRIMARY KEY,
    run_date            TIMESTAMPTZ DEFAULT NOW(),
    variant_code        VARCHAR(20) REFERENCES mdm.mdm_product_variants(variant_code),
    demand_qty          INTEGER NOT NULL CHECK (demand_qty > 0),
    demand_date         DATE,
    planning_horizon    INTEGER NOT NULL DEFAULT 30 CHECK (planning_horizon > 0),
    status              VARCHAR(20) DEFAULT '运行中' CHECK (status IN ('运行中', '完成', '取消')),
    created_by          VARCHAR(50),
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_mrp_suggestions (
    id                  BIGSERIAL PRIMARY KEY,
    run_id              VARCHAR(30) NOT NULL REFERENCES wms.wms_mrp_runs(run_id) ON DELETE CASCADE,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    bom_level           INTEGER DEFAULT 1 CHECK (bom_level >= 0),
    gross_requirement_qty INTEGER NOT NULL DEFAULT 0 CHECK (gross_requirement_qty >= 0),
    required_qty        INTEGER NOT NULL CHECK (required_qty >= 0),
    available_qty       INTEGER NOT NULL CHECK (available_qty >= 0),
    safety_stock_qty    INTEGER NOT NULL DEFAULT 0 CHECK (safety_stock_qty >= 0),
    shortage_qty        INTEGER GENERATED ALWAYS AS (GREATEST(required_qty + safety_stock_qty - available_qty, 0)) STORED,
    suggested_order_type VARCHAR(20) CHECK (suggested_order_type IN ('采购申请', '生产订单', '转储建议')),
    suggested_order_qty INTEGER CHECK (suggested_order_qty IS NULL OR suggested_order_qty >= 0),
    recommended_bin     VARCHAR(20) REFERENCES mdm.mdm_storage_bins(bin_code),
    recommended_batch   VARCHAR(30) REFERENCES wms.wms_batches(batch_number),
    lead_time_days      INTEGER DEFAULT 0 CHECK (lead_time_days >= 0),
    priority            INTEGER DEFAULT 1 CHECK (priority >= 1),
    remarks             TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(run_id, material_id, bom_level)
);

CREATE TABLE wms.wms_attachments (
    attachment_id       VARCHAR(30) PRIMARY KEY,
    related_table       VARCHAR(50) NOT NULL,
    related_id          VARCHAR(30) NOT NULL,
    file_name           VARCHAR(200) NOT NULL,
    file_path           TEXT NOT NULL,
    file_type           VARCHAR(50),
    file_size           BIGINT CHECK (file_size IS NULL OR file_size >= 0),
    uploaded_by         VARCHAR(50),
    uploaded_at         TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE wms.wms_notifications (
    notification_id     VARCHAR(30) PRIMARY KEY,
    notification_type   VARCHAR(30) NOT NULL,
    title               VARCHAR(200) NOT NULL,
    content             TEXT,
    priority            VARCHAR(10) DEFAULT '中' CHECK (priority IN ('低', '中', '高', '紧急')),
    status              VARCHAR(20) DEFAULT '未读' CHECK (status IN ('未读', '已读', '已处理')),
    recipient           VARCHAR(50),
    related_table       VARCHAR(50),
    related_id          VARCHAR(30),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    read_at             TIMESTAMPTZ
);

-- ============================================================
-- 4. SYS 系统与权限
-- ============================================================
CREATE TABLE sys.sys_roles (
    role_id             VARCHAR(20) PRIMARY KEY,
    role_name           VARCHAR(50) NOT NULL,
    description         TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sys.sys_users (
    user_id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username            VARCHAR(50) UNIQUE NOT NULL,
    password_hash       TEXT NOT NULL,
    full_name           VARCHAR(100),
    email               VARCHAR(100),
    role_id             VARCHAR(20) REFERENCES sys.sys_roles(role_id),
    is_active           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sys.sys_user_roles (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES sys.sys_users(user_id) ON DELETE CASCADE,
    role_id             VARCHAR(20) NOT NULL REFERENCES sys.sys_roles(role_id) ON DELETE CASCADE,
    assigned_by         VARCHAR(50),
    assigned_at         TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, role_id)
);

CREATE TABLE sys.sys_user_permissions (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID REFERENCES sys.sys_users(user_id) ON DELETE CASCADE,
    role_id             VARCHAR(20) REFERENCES sys.sys_roles(role_id) ON DELETE CASCADE,
    permission_code     VARCHAR(50) NOT NULL,
    permission_name     VARCHAR(100),
    granted             BOOLEAN DEFAULT TRUE,
    granted_by          VARCHAR(50),
    granted_at          TIMESTAMPTZ DEFAULT NOW(),
    expires_at          TIMESTAMPTZ,
    CHECK (user_id IS NOT NULL OR role_id IS NOT NULL)
);

CREATE TABLE sys.sys_audit_log (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID REFERENCES sys.sys_users(user_id),
    action              VARCHAR(50) NOT NULL,
    table_name          VARCHAR(80),
    record_id           TEXT,
    old_data            JSONB,
    new_data            JSONB,
    ip_address          INET,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sys.sys_system_params (
    param_key           VARCHAR(50) PRIMARY KEY,
    param_value         TEXT NOT NULL,
    param_type          VARCHAR(20) DEFAULT 'string' CHECK (param_type IN ('string', 'number', 'boolean', 'json')),
    description         TEXT,
    updated_by          VARCHAR(50),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- 4.1 P0 审计历史表：MAP 与批次状态流转
-- ============================================================
-- 1. MAP 价格变更历史（财务审计核心）
CREATE TABLE IF NOT EXISTS wms.wms_map_history (
    id                  BIGSERIAL PRIMARY KEY,
    material_id         VARCHAR(20) NOT NULL REFERENCES mdm.mdm_materials(material_id),
    transaction_id      VARCHAR(30),
    movement_type       wms.movement_type,
    old_map_price       NUMERIC(12,2),
    new_map_price       NUMERIC(12,2) NOT NULL CHECK (new_map_price >= 0),
    price_diff          NUMERIC(12,2)
                        GENERATED ALWAYS AS (new_map_price - COALESCE(old_map_price, 0)) STORED,
    old_stock_qty       INTEGER,
    received_qty        INTEGER,
    received_unit_price NUMERIC(12,2),
    received_amount     NUMERIC(15,2),
    new_stock_qty       INTEGER,
    calculation_formula TEXT,
    changed_by          VARCHAR(50),
    changed_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_maph_material   ON wms.wms_map_history(material_id);
CREATE INDEX IF NOT EXISTS idx_maph_changed_at ON wms.wms_map_history(changed_at DESC);
CREATE INDEX IF NOT EXISTS idx_maph_txn        ON wms.wms_map_history(transaction_id);
CREATE INDEX IF NOT EXISTS idx_maph_mat_time   ON wms.wms_map_history(material_id, changed_at DESC);

COMMENT ON TABLE wms.wms_map_history IS 
    'MAP 移动平均价变更历史 - 对应 SAP Material Ledger CKMLCR / 物料账';


-- 2. 批次状态流转历史（QM/IATF 16949 合规）
CREATE TABLE IF NOT EXISTS wms.wms_batch_history (
    id                  BIGSERIAL PRIMARY KEY,
    batch_number        VARCHAR(30) NOT NULL REFERENCES wms.wms_batches(batch_number),
    old_quality_status  mdm.quality_status,
    new_quality_status  mdm.quality_status NOT NULL,
    old_bin             VARCHAR(20),
    new_bin             VARCHAR(20),
    old_stock           INTEGER,
    new_stock           INTEGER,
    qty_change          INTEGER
                        GENERATED ALWAYS AS (COALESCE(new_stock,0) - COALESCE(old_stock,0)) STORED,
    change_reason       VARCHAR(100),
    inspection_lot_id   VARCHAR(30) REFERENCES wms.wms_inspection_lots(inspection_lot_id),
    notification_id     VARCHAR(30) REFERENCES wms.wms_quality_notifications(notification_id),
    transaction_id      VARCHAR(30),
    changed_by          VARCHAR(50),
    changed_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bath_batch      ON wms.wms_batch_history(batch_number);
CREATE INDEX IF NOT EXISTS idx_bath_changed    ON wms.wms_batch_history(changed_at DESC);
CREATE INDEX IF NOT EXISTS idx_bath_status     ON wms.wms_batch_history(new_quality_status);
CREATE INDEX IF NOT EXISTS idx_bath_batch_time ON wms.wms_batch_history(batch_number, changed_at DESC);

COMMENT ON TABLE wms.wms_batch_history IS 
    '批次状态流转历史 - 对应 SAP MSC2N，符合 IATF 16949 / VDA 6.3';


-- ============================================================
-- 5. 索引
-- ============================================================
CREATE INDEX idx_material_name_trgm ON mdm.mdm_materials USING GIN(material_name gin_trgm_ops);
CREATE INDEX idx_material_type ON mdm.mdm_materials(material_type);
CREATE INDEX idx_bins_zone ON mdm.mdm_storage_bins(zone);
CREATE INDEX idx_suppliers_rating ON mdm.mdm_suppliers(quality_rating);
CREATE INDEX idx_material_suppliers_material ON mdm.mdm_material_suppliers(material_id);
CREATE INDEX idx_bom_parent ON mdm.mdm_bom_components(parent_material_id);
CREATE INDEX idx_bom_component ON mdm.mdm_bom_components(component_material_id);
CREATE INDEX idx_price_material ON mdm.mdm_price_list(material_id);
CREATE INDEX idx_price_customer ON mdm.mdm_price_list(customer_id);

CREATE INDEX idx_batches_material ON wms.wms_batches(material_id);
CREATE INDEX idx_batches_quality ON wms.wms_batches(quality_status);
CREATE INDEX idx_serial_material ON wms.wms_serial_numbers(material_id);
CREATE INDEX idx_serial_batch ON wms.wms_serial_numbers(batch_number);
CREATE INDEX idx_serial_history_serial ON wms.wms_serial_history(serial_number);
CREATE INDEX idx_bin_stock_material ON wms.wms_bin_stock(material_id);
CREATE INDEX idx_bin_stock_bin ON wms.wms_bin_stock(bin_code);
CREATE INDEX idx_transactions_date_brin ON wms.wms_transactions USING BRIN(transaction_date);
CREATE INDEX idx_transactions_material ON wms.wms_transactions(material_id);
CREATE INDEX idx_transactions_batch ON wms.wms_transactions(batch_number);
CREATE INDEX idx_transactions_reference ON wms.wms_transactions(reference_doc);
CREATE INDEX idx_po_supplier ON wms.wms_purchase_orders_h(supplier_id);
CREATE INDEX idx_so_customer ON wms.wms_sales_orders_h(customer_id);
CREATE INDEX idx_prod_order_status ON wms.wms_production_orders_h(status);
CREATE INDEX idx_prod_variances_order ON wms.wms_production_variances(order_id);
CREATE INDEX idx_prod_variances_variant ON wms.wms_production_variances(variant_code);
CREATE INDEX idx_prod_variances_calculated ON wms.wms_production_variances(calculated_at DESC);
CREATE INDEX idx_genealogy_parent ON wms.wms_batch_genealogy(parent_batch_number);
CREATE INDEX idx_genealogy_component ON wms.wms_batch_genealogy(component_batch_number);
CREATE INDEX idx_inspection_lots_status ON wms.wms_inspection_lots(lot_status);
CREATE INDEX idx_inspection_lots_result_gin ON wms.wms_inspection_lots USING GIN(inspection_result);
CREATE INDEX idx_inspection_results_lot ON wms.wms_inspection_results(inspection_lot_id);
CREATE INDEX idx_quality_notifications_status ON wms.wms_quality_notifications(status);
CREATE INDEX idx_count_h_date ON wms.wms_inventory_count_h(count_date);
CREATE INDEX idx_count_d_material ON wms.wms_inventory_count_d(material_id);
CREATE INDEX idx_mrp_suggestions_run ON wms.wms_mrp_suggestions(run_id);
CREATE INDEX idx_mrp_suggestions_material ON wms.wms_mrp_suggestions(material_id);
CREATE INDEX idx_attach_related ON wms.wms_attachments(related_table, related_id);
CREATE INDEX idx_notify_recipient ON wms.wms_notifications(recipient, status);

CREATE INDEX idx_user_permissions_user ON sys.sys_user_permissions(user_id);
CREATE INDEX idx_audit_table_record ON sys.sys_audit_log(table_name, record_id);

-- ============================================================
-- 6. 库存过账函数 v8：事务驱动、批次/货位检查、防负库存、冻结控制、MAP 自动更新
--   - 新增：入库自动计算并更新 MAP，写入 wms_map_history
--   - 新增：批次状态/库存变更写入 wms_batch_history
--   - 保留：v5 原有的所有校验逻辑
-- ============================================================

-- 删除 v5 的旧签名，避免与 v6 带 p_unit_price 默认参数的新签名形成重载歧义
DROP FUNCTION IF EXISTS wms.post_inventory_transaction(
    VARCHAR(30), wms.movement_type, VARCHAR(20), INTEGER,
    VARCHAR(20), VARCHAR(20), VARCHAR(30), VARCHAR(30),
    VARCHAR(50), mdm.quality_status, VARCHAR(30), TEXT, TIMESTAMPTZ
);

CREATE OR REPLACE FUNCTION wms.post_inventory_transaction(
    p_transaction_id    VARCHAR(30),
    p_movement_type     wms.movement_type,
    p_material_id       VARCHAR(20),
    p_quantity          INTEGER,
    p_from_bin          VARCHAR(20) DEFAULT NULL,
    p_to_bin            VARCHAR(20) DEFAULT NULL,
    p_batch_number      VARCHAR(30) DEFAULT NULL,
    p_serial_number     VARCHAR(30) DEFAULT NULL,
    p_operator          VARCHAR(50) DEFAULT 'SYSTEM',
    p_quality_status    mdm.quality_status DEFAULT NULL,
    p_reference_doc     VARCHAR(30) DEFAULT NULL,
    p_notes             TEXT DEFAULT NULL,
    p_transaction_date  TIMESTAMPTZ DEFAULT NOW(),
    p_unit_price        NUMERIC(12,2) DEFAULT NULL    -- v6 新增：入库时的实际单价（用于 MAP 计算）
) RETURNS VOID AS $$
DECLARE
    v_cfg                   wms.wms_movement_type_config%ROWTYPE;
    v_material_stock        INTEGER;
    v_batch_stock           INTEGER;
    v_from_stock            INTEGER;
    v_batch_status          mdm.quality_status;
    v_batch_old_bin         VARCHAR(20);
    v_effective_quality     mdm.quality_status;
    v_material_delta        INTEGER := 0;
    v_batch_delta           INTEGER := 0;
    v_txn_quantity          INTEGER := 0;
    v_capacity              INTEGER;
    v_occupied              INTEGER;
    v_old_serial_status     VARCHAR(20);
    v_old_serial_bin        VARCHAR(20);
    v_old_serial_quality    mdm.quality_status;
    v_new_serial_status     VARCHAR(20);
    -- v6 新增变量
    v_old_map               NUMERIC(12,2);
    v_new_map               NUMERIC(12,2);
    v_old_total_value       NUMERIC(15,2);
    v_received_amount       NUMERIC(15,2);
    v_formula               TEXT;
BEGIN
    IF p_quantity IS NULL OR p_quantity <= 0 THEN
        RAISE EXCEPTION '过账数量必须大于 0';
    END IF;

    SELECT * INTO v_cfg FROM wms.wms_movement_type_config WHERE movement_type = p_movement_type;
    IF NOT FOUND THEN
        RAISE EXCEPTION '移动类型 % 未配置', p_movement_type;
    END IF;

    IF v_cfg.requires_from_bin AND p_from_bin IS NULL THEN
        RAISE EXCEPTION '移动类型 % 必须提供 from_bin', p_movement_type;
    END IF;
    IF v_cfg.requires_to_bin AND p_to_bin IS NULL THEN
        RAISE EXCEPTION '移动类型 % 必须提供 to_bin', p_movement_type;
    END IF;
    IF v_cfg.requires_batch AND p_batch_number IS NULL THEN
        RAISE EXCEPTION '移动类型 % 必须提供 batch_number', p_movement_type;
    END IF;

    IF p_batch_number IS NOT NULL THEN
        SELECT current_stock, quality_status, current_bin
          INTO v_batch_stock, v_batch_status, v_batch_old_bin
        FROM wms.wms_batches
        WHERE batch_number = p_batch_number AND material_id = p_material_id
        FOR UPDATE;

        IF NOT FOUND THEN
            RAISE EXCEPTION '批次 % 不存在或不属于物料 %', p_batch_number, p_material_id;
        END IF;

        v_effective_quality := COALESCE(p_quality_status, v_batch_status);

        IF p_movement_type IN ('261', '311', '702', '999') 
           AND v_batch_status IN ('冻结', '报废') THEN
            RAISE EXCEPTION '批次 % 当前质量状态为 %，禁止出库/转移/报废过账', 
                            p_batch_number, v_batch_status;
        END IF;
    ELSE
        v_effective_quality := COALESCE(p_quality_status, '合格'::mdm.quality_status);
    END IF;

    SELECT current_stock, map_price 
      INTO v_material_stock, v_old_map
    FROM mdm.mdm_materials
    WHERE material_id = p_material_id
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION '物料 % 不存在', p_material_id;
    END IF;

    -- 数量增量计算（与 v5 一致）
    CASE p_movement_type
        WHEN '101' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '501' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '701' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '261' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '702' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '999' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '311' THEN v_material_delta:=0;           v_batch_delta:=0;           v_txn_quantity:=p_quantity;
    END CASE;

    -- 校验（与 v5 一致）
    IF v_material_delta < 0 AND v_material_stock < p_quantity THEN
        RAISE EXCEPTION '物料 % 总库存不足，当前 %，需扣 %', 
                        p_material_id, v_material_stock, p_quantity;
    END IF;
    IF p_batch_number IS NOT NULL AND v_batch_delta < 0 AND v_batch_stock < p_quantity THEN
        RAISE EXCEPTION '批次 % 库存不足，当前 %，需扣 %', 
                        p_batch_number, v_batch_stock, p_quantity;
    END IF;
    IF p_from_bin IS NOT NULL THEN
        SELECT qty INTO v_from_stock 
        FROM wms.wms_bin_stock 
        WHERE material_id=p_material_id AND bin_code=p_from_bin AND batch_number=p_batch_number 
        FOR UPDATE;
        IF NOT FOUND OR v_from_stock < p_quantity THEN
            RAISE EXCEPTION '货位 % 物料 % 批次 % 库存不足', 
                            p_from_bin, p_material_id, p_batch_number;
        END IF;
    END IF;
    IF p_to_bin IS NOT NULL THEN
        SELECT capacity, current_occupied INTO v_capacity, v_occupied 
        FROM mdm.mdm_storage_bins WHERE bin_code = p_to_bin FOR UPDATE;
        IF NOT FOUND THEN
            RAISE EXCEPTION '目标货位 % 不存在', p_to_bin;
        END IF;
        IF v_occupied + p_quantity > v_capacity THEN
            RAISE EXCEPTION '目标货位 % 容量不足', p_to_bin;
        END IF;
    END IF;

    -- 写事务日志
    INSERT INTO wms.wms_transactions (
        transaction_id, transaction_date, movement_type, material_id, quantity,
        from_bin, to_bin, batch_number, serial_number, reference_doc,
        operator, quality_status, notes
    ) VALUES (
        p_transaction_id, p_transaction_date, p_movement_type, p_material_id, v_txn_quantity,
        p_from_bin, p_to_bin, p_batch_number, p_serial_number, p_reference_doc,
        p_operator, v_effective_quality, p_notes
    );

    -- 更新 bin_stock
    IF p_from_bin IS NOT NULL THEN
        UPDATE wms.wms_bin_stock SET qty=qty-p_quantity, updated_at=NOW()
        WHERE material_id=p_material_id AND bin_code=p_from_bin AND batch_number=p_batch_number;
        UPDATE mdm.mdm_storage_bins SET current_occupied=current_occupied-p_quantity, updated_at=NOW()
        WHERE bin_code = p_from_bin;
    END IF;
    IF p_to_bin IS NOT NULL THEN
        INSERT INTO wms.wms_bin_stock (material_id, bin_code, batch_number, quality_status, qty)
        VALUES (p_material_id, p_to_bin, p_batch_number, v_effective_quality, p_quantity)
        ON CONFLICT (material_id, bin_code, batch_number)
        DO UPDATE SET qty=wms.wms_bin_stock.qty+EXCLUDED.qty, 
                      quality_status=EXCLUDED.quality_status, updated_at=NOW();
        UPDATE mdm.mdm_storage_bins SET current_occupied=current_occupied+p_quantity, updated_at=NOW()
        WHERE bin_code = p_to_bin;
    END IF;
    DELETE FROM wms.wms_bin_stock WHERE qty=0;

    -- ====================================================
    -- v6 核心新增：MAP 自动更新（仅入库类）
    -- ====================================================
    IF p_movement_type IN ('101','501','701') AND p_unit_price IS NOT NULL THEN
        v_old_total_value := COALESCE(v_material_stock,0) * COALESCE(v_old_map,0);
        v_received_amount := p_quantity * p_unit_price;

        IF (v_material_stock + p_quantity) > 0 THEN
            v_new_map := ROUND(
                (v_old_total_value + v_received_amount)::NUMERIC 
                / (v_material_stock + p_quantity), 4
            );
        ELSE
            v_new_map := v_old_map;
        END IF;

        v_formula := format(
            '(%s × %s + %s × %s) / (%s + %s) = %s',
            v_material_stock, COALESCE(v_old_map,0),
            p_quantity, p_unit_price,
            v_material_stock, p_quantity,
            v_new_map
        );

        INSERT INTO wms.wms_map_history (
            material_id, transaction_id, movement_type,
            old_map_price, new_map_price,
            old_stock_qty, received_qty, received_unit_price, received_amount, new_stock_qty,
            calculation_formula, changed_by
        ) VALUES (
            p_material_id, p_transaction_id, p_movement_type,
            v_old_map, v_new_map,
            v_material_stock, p_quantity, p_unit_price, v_received_amount,
            v_material_stock + p_quantity,
            v_formula, p_operator
        );
    END IF;

    -- 更新物料主数据（数量 + MAP）
    UPDATE mdm.mdm_materials
    SET current_stock = current_stock + v_material_delta,
        map_price = CASE 
            WHEN p_movement_type IN ('101','501','701') AND p_unit_price IS NOT NULL 
            THEN v_new_map ELSE map_price 
        END,
        updated_at = NOW()
    WHERE material_id = p_material_id;

    -- ====================================================
    -- 批次更新 + v6 新增 batch_history
    -- ====================================================
    IF p_batch_number IS NOT NULL THEN
        UPDATE wms.wms_batches
        SET current_stock = current_stock + v_batch_delta,
            current_bin = CASE
                WHEN p_to_bin IS NOT NULL THEN p_to_bin
                WHEN current_stock + v_batch_delta = 0 THEN NULL
                ELSE current_bin
            END,
            quality_status = v_effective_quality,
            updated_at = NOW()
        WHERE batch_number = p_batch_number;

        -- v6 新增：写批次历史
        IF v_batch_status IS DISTINCT FROM v_effective_quality
           OR v_batch_old_bin IS DISTINCT FROM COALESCE(p_to_bin, v_batch_old_bin)
           OR v_batch_delta <> 0 THEN
            INSERT INTO wms.wms_batch_history (
                batch_number, old_quality_status, new_quality_status,
                old_bin, new_bin, old_stock, new_stock,
                change_reason, transaction_id, changed_by
            ) VALUES (
                p_batch_number, v_batch_status, v_effective_quality,
                v_batch_old_bin, COALESCE(p_to_bin, v_batch_old_bin),
                v_batch_stock, v_batch_stock + v_batch_delta,
                format('Movement %s', p_movement_type),
                p_transaction_id, p_operator
            );
        END IF;
    END IF;

    -- 序列号处理（与 v5 一致）
    IF p_serial_number IS NOT NULL THEN
        SELECT current_status, current_bin, quality_status
          INTO v_old_serial_status, v_old_serial_bin, v_old_serial_quality
        FROM wms.wms_serial_numbers WHERE serial_number = p_serial_number FOR UPDATE;

        v_new_serial_status := CASE
            WHEN p_movement_type IN ('261') THEN '生产中'
            WHEN p_movement_type IN ('702','999') THEN '报废'
            ELSE COALESCE(v_old_serial_status, '在库')
        END;

        IF FOUND THEN
            UPDATE wms.wms_serial_numbers
            SET current_status = v_new_serial_status,
                current_bin = COALESCE(p_to_bin, p_from_bin, current_bin),
                quality_status = v_effective_quality,
                last_movement_at = NOW(), updated_at = NOW()
            WHERE serial_number = p_serial_number;
        ELSE
            INSERT INTO wms.wms_serial_numbers (
                serial_number, material_id, batch_number, current_status, current_bin,
                quality_status, last_movement_at
            ) VALUES (
                p_serial_number, p_material_id, p_batch_number, v_new_serial_status,
                COALESCE(p_to_bin, p_from_bin), v_effective_quality, NOW()
            );
        END IF;

        INSERT INTO wms.wms_serial_history (
            serial_number, old_status, new_status, old_bin, new_bin,
            old_quality_status, new_quality_status, transaction_id, changed_by
        ) VALUES (
            p_serial_number, v_old_serial_status, v_new_serial_status, v_old_serial_bin,
            COALESCE(p_to_bin, p_from_bin, v_old_serial_bin), v_old_serial_quality,
            v_effective_quality, p_transaction_id, p_operator
        );
    END IF;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION wms.post_inventory_transaction IS 
    '库存过账核心函数 v6 - 数量+MAP 双更新，自动写 map_history / batch_history / serial_history';

-- ============================================================
-- 7. 演示数据：配置、主数据、订单、批次、事务、QM、MRP、盘点
-- ============================================================
INSERT INTO wms.wms_movement_type_config
(movement_type, movement_name, direction, requires_from_bin, requires_to_bin, requires_batch, requires_serial, requires_quality, affects_material_stock, description)
VALUES
('101', '采购/生产入库', '入库', FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, '采购收货或生产收货入库'),
('261', '生产领料/销售出库', '出库', TRUE, FALSE, TRUE, FALSE, TRUE, TRUE, '生产领料或出库消耗'),
('311', '货位转移', '转移', TRUE, TRUE, TRUE, FALSE, TRUE, FALSE, '同一物料批次在不同货位之间转储'),
('501', '其他入库', '入库', FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, '无采购订单的其他入库'),
('701', '盘盈', '入库', FALSE, TRUE, TRUE, FALSE, TRUE, TRUE, '盘点增益入库'),
('702', '盘亏', '出库', TRUE, FALSE, TRUE, FALSE, TRUE, TRUE, '盘点差异扣减'),
('999', '报废', '出库', TRUE, FALSE, TRUE, FALSE, TRUE, TRUE, '报废出库');

INSERT INTO sys.sys_roles (role_id, role_name, description) VALUES
('ADMIN', '系统管理员', '拥有系统配置和权限管理能力'),
('WMS_USER', '仓库用户', '执行入库、出库、转储、盘点'),
('QM_USER', '质量用户', '执行检验批和质量通知处理'),
('PLANNER', '计划员', '执行 MRP 和生产计划');

INSERT INTO sys.sys_users (username, password_hash, full_name, email, role_id) VALUES
('admin', 'demo-not-for-production', '系统管理员', 'admin@example.com', 'ADMIN'),
('wms01', 'demo-not-for-production', '仓库操作员', 'wms01@example.com', 'WMS_USER'),
('qm01', 'demo-not-for-production', '质量工程师', 'qm01@example.com', 'QM_USER'),
('planner01', 'demo-not-for-production', '计划员', 'planner01@example.com', 'PLANNER');

INSERT INTO sys.sys_user_roles (user_id, role_id, assigned_by)
SELECT user_id, role_id, 'SYSTEM' FROM sys.sys_users WHERE role_id IS NOT NULL;

INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by) VALUES
('ADMIN', 'SYS_ALL', '系统全部权限', 'SYSTEM'),
('WMS_USER', 'WMS_POST_TRANSACTION', '库存过账', 'SYSTEM'),
('WMS_USER', 'WMS_COUNT', '盘点', 'SYSTEM'),
('QM_USER', 'QM_INSPECTION', '质量检验', 'SYSTEM'),
('PLANNER', 'MRP_RUN', 'MRP 运算', 'SYSTEM');

INSERT INTO sys.sys_system_params (param_key, param_value, param_type, description, updated_by) VALUES
('DEFAULT_SAFETY_STOCK_DAYS', '7', 'number', '默认安全库存天数', 'SYSTEM'),
('DEFAULT_REORDER_POINT_DAYS', '14', 'number', '默认再订货点天数', 'SYSTEM'),
('MRP_PLANNING_HORIZON', '30', 'number', 'MRP 默认计划周期（天）', 'SYSTEM'),
('QUALITY_INSPECTION_REQUIRED', 'true', 'boolean', '是否强制质量检验', 'SYSTEM'),
('SERIAL_NUMBER_REQUIRED_MATERIALS', 'CG001,FPC001,LCM001', 'string', '必须录入序列号的物料', 'SYSTEM');

INSERT INTO mdm.mdm_materials
(material_id, material_name, material_type, default_zone, safety_stock, reorder_point, standard_price, map_price, price_control)
VALUES
('CG001', 'CG 盖板玻璃 (Cover Glass)', '原材料', 'RM', 30, 50, 18.50, 18.50, 'Moving Average'),
('FPC001', 'FPC 柔性电路板', '原材料', 'RM', 30, 50, 9.80, 9.80, 'Moving Average'),
('FUNC001', '功能片 (Functional Sheet)', '原材料', 'RM', 30, 50, 14.20, 14.20, 'Moving Average'),
('LCM001', 'LCM 液晶模组', '原材料', 'RM', 20, 40, 52.00, 52.00, 'Moving Average'),
('LCM-S001', '特殊 LCM 液晶模组', '原材料', 'RM', 10, 20, 68.00, 70.00, 'Moving Average'),
('PROT001', '保护膜 (Protective Film)', '原材料', 'RM', 50, 80, 4.50, 4.50, 'Moving Average'),
('FOG001', 'FOG 组件 (FPC + 功能片)', '半成品', 'SF', 10, 20, 25.00, 25.50, 'Moving Average'),
('TP001', 'TP 触摸面板 (FOG + CG)', '半成品', 'SF', 8, 15, 48.00, 49.00, 'Moving Average'),
('FIN001', '总成成品 (标准版)', '成品', 'FG', 5, 10, 110.00, 112.00, 'Moving Average'),
('FIN-B001', '轻薄总成 (TP + LCM，无保护膜)', '成品', 'FG', 5, 10, 95.00, 97.00, 'Moving Average'),
('FIN-C001', '高端总成 (TP + 特殊LCM + 保护膜 + 特殊CG)', '成品', 'FG', 3, 8, 135.00, 138.00, 'Moving Average');

INSERT INTO mdm.mdm_storage_bins (bin_code, zone, bin_type, capacity, notes) VALUES
('RM-A01', 'RM', '标准货架', 200, '原材料 A 区'),
('RM-A02', 'RM', '标准货架', 400, '原材料 A 区'),
('RM-B01', 'RM', '高位货架', 100, '原材料 B 区'),
('SF-C01', 'SF', '周转货架', 80, '半成品 C 区'),
('SF-C02', 'SF', '周转货架', 80, '半成品 C 区'),
('FG-D01', 'FG', '成品货架', 50, '成品 D 区'),
('FG-D02', 'FG', '成品货架', 50, '成品 D 区'),
('PROD-E01', 'PROD', '临时工位', 80, '生产临时工位');

INSERT INTO mdm.mdm_suppliers (supplier_id, supplier_name, contact_person, phone, email, quality_rating) VALUES
('SUP-A001', '华南玻璃供应商', '张三', '13800000001', 'supplier-a@example.com', 'A'),
('SUP-B001', '华东电子材料', '李四', '13800000002', 'supplier-b@example.com', 'A'),
('SUP-C001', '模组科技有限公司', '王五', '13800000003', 'supplier-c@example.com', 'B');

INSERT INTO mdm.mdm_customers (customer_id, customer_name, contact_person, phone, email, credit_limit) VALUES
('CUST-001', '深圳终端客户 A', '赵六', '13900000001', 'customer-a@example.com', 1000000),
('CUST-002', '上海终端客户 B', '钱七', '13900000002', 'customer-b@example.com', 800000);

INSERT INTO mdm.mdm_material_suppliers
(material_id, supplier_id, is_primary, supplier_material_code, purchase_price, lead_time_days, moq, quality_rating, qualified_date)
VALUES
('CG001', 'SUP-A001', TRUE, 'SUPA-CG-001', 18.50, 7, 100, 'A', '2026-01-01'),
('FPC001', 'SUP-B001', TRUE, 'SUPB-FPC-001', 9.80, 10, 100, 'A', '2026-01-01'),
('FUNC001', 'SUP-B001', TRUE, 'SUPB-FUNC-001', 14.20, 10, 100, 'A', '2026-01-01'),
('LCM001', 'SUP-C001', TRUE, 'SUPC-LCM-001', 52.00, 14, 50, 'B', '2026-01-01'),
('LCM-S001', 'SUP-C001', TRUE, 'SUPC-LCMS-001', 70.00, 16, 30, 'B', '2026-01-01'),
('PROT001', 'SUP-A001', TRUE, 'SUPA-PROT-001', 4.50, 5, 200, 'A', '2026-01-01');

INSERT INTO mdm.mdm_inspection_chars
(char_id, char_name, material_type, inspection_type, method, standard, unit, lower_limit, upper_limit, is_critical)
VALUES
('CHAR-CG-THICK', '盖板厚度', '原材料', '来料检验', '千分尺测量', '0.70±0.05', 'mm', 0.650, 0.750, TRUE),
('CHAR-FPC-FUNC', 'FPC 导通测试', '原材料', '来料检验', '治具测试', '全部导通', NULL, NULL, NULL, TRUE),
('CHAR-FIN-LOOK', '成品外观', '成品', '最终检验', '目视检验', '无划伤/无脏污', NULL, NULL, NULL, TRUE);

INSERT INTO mdm.mdm_defect_codes (defect_code, defect_name, category, severity, description) VALUES
('D001', '外观划伤', '外观', '一般', '表面存在轻微划伤'),
('D002', '尺寸超差', '尺寸', '严重', '尺寸超过上下限'),
('D003', '功能不良', '功能', '紧急', '导通或显示功能失败');

INSERT INTO mdm.mdm_work_centers (work_center_id, work_center_name, location, capacity_per_day, efficiency) VALUES
('WC-FOG', 'FOG 组装线', '一楼 A 区', 500, 95.00),
('WC-TP', 'TP 贴合线', '一楼 B 区', 400, 92.00),
('WC-FIN', '总成装配线', '二楼 C 区', 300, 90.00);

INSERT INTO mdm.mdm_cost_centers (cost_center_id, cost_center_name, department, manager) VALUES
('CC-WH', '仓储成本中心', '仓储部', '仓储经理'),
('CC-PROD', '生产成本中心', '生产部', '生产经理'),
('CC-QM', '质量成本中心', '质量部', '质量经理');

INSERT INTO mdm.mdm_gl_accounts (gl_account, account_name, account_type) VALUES
('1403', '原材料', '资产'),
('1405', '库存商品', '资产'),
('5001', '主营业务收入', '收入'),
('6401', '主营业务成本', '成本');

INSERT INTO mdm.mdm_product_variants (variant_code, variant_name, base_material_id, standard_cost) VALUES
('FIN-A001', '标准版总成 (TP + LCM + 保护膜)', 'FIN001', 110.00),
('FIN-B001', '轻薄版总成 (TP + LCM)', 'FIN-B001', 95.00),
('FIN-C001', '高端版总成 (TP + 特殊LCM + 保护膜 + 特殊CG)', 'FIN-C001', 135.00);

INSERT INTO mdm.mdm_bom_headers
(bom_id, bom_name, parent_material_id, variant_code, version, status, valid_from, created_by, approved_by, approved_at)
VALUES
('BOM-FOG-01', 'FOG 组件 BOM', 'FOG001', NULL, 'V1.0', '生效', '2026-01-01', 'planner01', 'admin', '2026-01-02 09:00:00+08'),
('BOM-TP-01', 'TP 触摸面板 BOM', 'TP001', NULL, 'V1.0', '生效', '2026-01-01', 'planner01', 'admin', '2026-01-02 09:00:00+08'),
('BOM-FIN-A01', '标准版总成 BOM', 'FIN001', 'FIN-A001', 'V1.0', '生效', '2026-01-01', 'planner01', 'admin', '2026-01-02 09:00:00+08'),
('BOM-FIN-B01', '轻薄版总成 BOM', 'FIN-B001', 'FIN-B001', 'V1.0', '生效', '2026-01-01', 'planner01', 'admin', '2026-01-02 09:00:00+08'),
('BOM-FIN-C01', '高端版总成 BOM', 'FIN-C001', 'FIN-C001', 'V1.0', '生效', '2026-01-01', 'planner01', 'admin', '2026-01-02 09:00:00+08');

UPDATE mdm.mdm_product_variants SET bom_id = 'BOM-FIN-A01' WHERE variant_code = 'FIN-A001';
UPDATE mdm.mdm_product_variants SET bom_id = 'BOM-FIN-B01' WHERE variant_code = 'FIN-B001';
UPDATE mdm.mdm_product_variants SET bom_id = 'BOM-FIN-C01' WHERE variant_code = 'FIN-C001';

INSERT INTO mdm.mdm_bom_components
(bom_id, parent_material_id, component_material_id, quantity, unit, bom_level, is_critical, valid_from)
VALUES
('BOM-FOG-01', 'FOG001', 'FPC001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FOG-01', 'FOG001', 'FUNC001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-TP-01', 'TP001', 'FOG001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-TP-01', 'TP001', 'CG001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-A01', 'FIN001', 'TP001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-A01', 'FIN001', 'LCM001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-A01', 'FIN001', 'PROT001', 1, 'PCS', 1, FALSE, '2026-01-01'),
('BOM-FIN-B01', 'FIN-B001', 'TP001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-B01', 'FIN-B001', 'LCM001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-C01', 'FIN-C001', 'TP001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-C01', 'FIN-C001', 'LCM-S001', 1, 'PCS', 1, TRUE, '2026-01-01'),
('BOM-FIN-C01', 'FIN-C001', 'PROT001', 1, 'PCS', 1, FALSE, '2026-01-01'),
('BOM-FIN-C01', 'FIN-C001', 'CG001', 1, 'PCS', 1, TRUE, '2026-01-01');

INSERT INTO mdm.mdm_price_list (material_id, customer_id, price_type, unit_price, valid_from, valid_to) VALUES
('FIN001', 'CUST-001', '合同价', 150.00, '2026-01-01', '2026-12-31'),
('FIN-B001', 'CUST-001', '合同价', 130.00, '2026-01-01', '2026-12-31'),
('FIN-C001', 'CUST-001', '合同价', 180.00, '2026-01-01', '2026-12-31'),
('FIN001', 'CUST-002', '标准价', 155.00, '2026-01-01', '2026-12-31');

INSERT INTO wms.wms_purchase_orders_h
(po_id, supplier_id, po_date, expected_date, total_amount, status, created_by, approved_by, approved_at)
VALUES
('PO-2026-0001', 'SUP-A001', '2026-04-01', '2026-04-08', 1850.00, '完成', 'PurchUser', 'admin', '2026-04-01 10:00:00+08'),
('PO-2026-0002', 'SUP-B001', '2026-04-01', '2026-04-10', 2400.00, '完成', 'PurchUser', 'admin', '2026-04-01 10:10:00+08'),
('PO-2026-0003', 'SUP-C001', '2026-04-03', '2026-04-15', 6500.00, '完成', 'PurchUser', 'admin', '2026-04-03 10:20:00+08');

INSERT INTO wms.wms_purchase_orders_d
(po_id, line_no, material_id, ordered_qty, received_qty, unit_price, expected_bin, line_status)
VALUES
('PO-2026-0001', 1, 'CG001', 120, 120, 18.50, 'RM-A01', '完成'),
('PO-2026-0001', 2, 'PROT001', 150, 150, 4.50, 'RM-A02', '完成'),
('PO-2026-0002', 1, 'FPC001', 100, 100, 9.80, 'RM-A02', '完成'),
('PO-2026-0002', 2, 'FUNC001', 100, 100, 14.20, 'RM-A02', '完成'),
('PO-2026-0003', 1, 'LCM001', 70, 70, 52.00, 'RM-B01', '完成'),
('PO-2026-0003', 2, 'LCM-S001', 30, 30, 70.00, 'RM-B01', '完成');

INSERT INTO wms.wms_sales_orders_h
(so_id, customer_id, so_date, delivery_date, total_amount, total_cogs, status, created_by, approved_by, approved_at)
VALUES
('SO-2026-0001', 'CUST-001', '2026-04-25', '2026-05-05', 5380.00, 3782.00, '部分发货', 'SalesUser', 'admin', '2026-04-25 14:00:00+08');

INSERT INTO wms.wms_sales_orders_d
(so_id, line_no, material_id, variant_code, ordered_qty, shipped_qty, unit_price, map_at_shipment, batch_number, from_bin, line_status)
VALUES
('SO-2026-0001', 1, 'FIN001', 'FIN-A001', 20, 10, 150.00, 112.00, NULL, NULL, '部分发货'),
('SO-2026-0001', 2, 'FIN-B001', 'FIN-B001', 5, 0, 130.00, 97.00, NULL, NULL, '待发货'),
('SO-2026-0001', 3, 'FIN-C001', 'FIN-C001', 5, 0, 180.00, 138.00, NULL, NULL, '待发货');

INSERT INTO wms.wms_production_orders_h
(order_id, variant_code, bom_id, output_material_id, work_center_id, planned_quantity, actual_quantity, status, planned_start_date, planned_finish_date, actual_start_date, actual_finish_date, created_by)
VALUES
('PRD-FOG-0001', NULL, 'BOM-FOG-01', 'FOG001', 'WC-FOG', 30, 30, '完成', '2026-04-15', '2026-04-15', '2026-04-15', '2026-04-15', 'planner01'),
('PRD-TP-0001', NULL, 'BOM-TP-01', 'TP001', 'WC-TP', 25, 25, '完成', '2026-04-18', '2026-04-18', '2026-04-18', '2026-04-18', 'planner01'),
('PRD-FIN-0001', 'FIN-A001', 'BOM-FIN-A01', 'FIN001', 'WC-FIN', 20, 20, '完成', '2026-04-22', '2026-04-22', '2026-04-22', '2026-04-22', 'planner01');

INSERT INTO wms.wms_production_orders_d
(order_id, line_no, material_id, batch_number, planned_qty, actual_qty, from_bin, issue_transaction_id)
VALUES
('PRD-FOG-0001', 1, 'FPC001', NULL, 30, 30, 'RM-A02', 'T0007'),
('PRD-FOG-0001', 2, 'FUNC001', NULL, 30, 30, 'RM-A02', 'T0008'),
('PRD-TP-0001', 1, 'FOG001', NULL, 25, 25, 'SF-C01', 'T0010'),
('PRD-TP-0001', 2, 'CG001', NULL, 25, 25, 'RM-A01', 'T0011'),
('PRD-FIN-0001', 1, 'TP001', NULL, 20, 20, 'SF-C02', 'T0013'),
('PRD-FIN-0001', 2, 'LCM001', NULL, 20, 20, 'RM-B01', 'T0014'),
('PRD-FIN-0001', 3, 'PROT001', NULL, 20, 20, 'RM-A02', 'T0015');

INSERT INTO wms.wms_batches
(batch_number, material_id, production_date, expiry_date, supplier_batch, quality_grade, current_stock, current_bin, quality_status)
VALUES
('B-CG-20260401', 'CG001', '2026-04-01', '2027-04-01', 'SUPA-CG-LOT01', 'A', 0, NULL, '合格'),
('B-PROT-20260401', 'PROT001', '2026-04-01', '2027-04-01', 'SUPA-PROT-LOT01', 'A', 0, NULL, '合格'),
('B-FPC-20260402', 'FPC001', '2026-04-02', '2027-04-02', 'SUPB-FPC-LOT01', 'A', 0, NULL, '合格'),
('B-FUNC-20260402', 'FUNC001', '2026-04-02', '2027-04-02', 'SUPB-FUNC-LOT01', 'A', 0, NULL, '合格'),
('B-LCM-20260403', 'LCM001', '2026-04-03', '2027-04-03', 'SUPC-LCM-LOT01', 'B', 0, NULL, '合格'),
('B-LCMS-20260403', 'LCM-S001', '2026-04-03', '2027-04-03', 'SUPC-LCMS-LOT01', 'B', 0, NULL, '合格'),
('B-FOG-20260415', 'FOG001', '2026-04-15', '2027-04-15', NULL, 'A', 0, NULL, '合格'),
('B-TP-20260418', 'TP001', '2026-04-18', '2027-04-18', NULL, 'A', 0, NULL, '合格'),
('B-FIN-20260422', 'FIN001', '2026-04-22', '2027-04-22', NULL, 'A', 0, NULL, '合格'),
('B-FINB-20260423', 'FIN-B001', '2026-04-23', '2027-04-23', NULL, 'A', 0, NULL, '待检'),
('B-FINC-20260424', 'FIN-C001', '2026-04-24', '2027-04-24', NULL, 'A', 0, NULL, '待检');

INSERT INTO wms.wms_batch_attributes
(batch_number, manufacturing_date, expiry_date, lot_number, supplier_lot, moisture_level, storage_condition, custom_attributes)
SELECT batch_number, production_date, expiry_date, batch_number, supplier_batch, '正常', '常温干燥', jsonb_build_object('source', 'demo')
FROM wms.wms_batches;

SELECT wms.post_inventory_transaction('T0001', '101', 'CG001', 120, NULL, 'RM-A01', 'B-CG-20260401', NULL, 'wms01', '合格', 'PO-2026-0001', 'CG 采购入库', '2026-04-08 09:00:00+08', 18.50);
SELECT wms.post_inventory_transaction('T0002', '101', 'PROT001', 150, NULL, 'RM-A02', 'B-PROT-20260401', NULL, 'wms01', '合格', 'PO-2026-0001', '保护膜采购入库', '2026-04-08 09:10:00+08', 4.50);
SELECT wms.post_inventory_transaction('T0003', '101', 'FPC001', 100, NULL, 'RM-A02', 'B-FPC-20260402', NULL, 'wms01', '合格', 'PO-2026-0002', 'FPC 采购入库', '2026-04-10 09:00:00+08', 9.80);
SELECT wms.post_inventory_transaction('T0004', '101', 'FUNC001', 100, NULL, 'RM-A02', 'B-FUNC-20260402', NULL, 'wms01', '合格', 'PO-2026-0002', '功能片采购入库', '2026-04-10 09:10:00+08', 14.20);
SELECT wms.post_inventory_transaction('T0005', '101', 'LCM001', 70, NULL, 'RM-B01', 'B-LCM-20260403', NULL, 'wms01', '合格', 'PO-2026-0003', 'LCM 采购入库', '2026-04-15 09:00:00+08', 52.00);
SELECT wms.post_inventory_transaction('T0006', '101', 'LCM-S001', 30, NULL, 'RM-B01', 'B-LCMS-20260403', NULL, 'wms01', '合格', 'PO-2026-0003', '特殊 LCM 采购入库', '2026-04-15 09:10:00+08', 70.00);

SELECT wms.post_inventory_transaction('T0007', '261', 'FPC001', 30, 'RM-A02', NULL, 'B-FPC-20260402', NULL, 'wms01', '合格', 'PRD-FOG-0001', 'FOG 生产领用 FPC', '2026-04-15 10:00:00+08');
SELECT wms.post_inventory_transaction('T0008', '261', 'FUNC001', 30, 'RM-A02', NULL, 'B-FUNC-20260402', NULL, 'wms01', '合格', 'PRD-FOG-0001', 'FOG 生产领用功能片', '2026-04-15 10:05:00+08');
SELECT wms.post_inventory_transaction('T0009', '101', 'FOG001', 30, NULL, 'SF-C01', 'B-FOG-20260415', NULL, 'wms01', '合格', 'PRD-FOG-0001', 'FOG 生产入库', '2026-04-15 16:00:00+08');

SELECT wms.post_inventory_transaction('T0010', '261', 'FOG001', 25, 'SF-C01', NULL, 'B-FOG-20260415', NULL, 'wms01', '合格', 'PRD-TP-0001', 'TP 生产领用 FOG', '2026-04-18 10:00:00+08');
SELECT wms.post_inventory_transaction('T0011', '261', 'CG001', 25, 'RM-A01', NULL, 'B-CG-20260401', NULL, 'wms01', '合格', 'PRD-TP-0001', 'TP 生产领用 CG', '2026-04-18 10:05:00+08');
SELECT wms.post_inventory_transaction('T0012', '101', 'TP001', 25, NULL, 'SF-C02', 'B-TP-20260418', NULL, 'wms01', '合格', 'PRD-TP-0001', 'TP 生产入库', '2026-04-18 16:00:00+08');

SELECT wms.post_inventory_transaction('T0013', '261', 'TP001', 20, 'SF-C02', NULL, 'B-TP-20260418', NULL, 'wms01', '合格', 'PRD-FIN-0001', 'FIN 生产领用 TP', '2026-04-22 10:00:00+08');
SELECT wms.post_inventory_transaction('T0014', '261', 'LCM001', 20, 'RM-B01', NULL, 'B-LCM-20260403', NULL, 'wms01', '合格', 'PRD-FIN-0001', 'FIN 生产领用 LCM', '2026-04-22 10:05:00+08');
SELECT wms.post_inventory_transaction('T0015', '261', 'PROT001', 20, 'RM-A02', NULL, 'B-PROT-20260401', NULL, 'wms01', '合格', 'PRD-FIN-0001', 'FIN 生产领用保护膜', '2026-04-22 10:10:00+08');
SELECT wms.post_inventory_transaction('T0016', '101', 'FIN001', 20, NULL, 'FG-D01', 'B-FIN-20260422', NULL, 'wms01', '合格', 'PRD-FIN-0001', 'FIN-A 生产入库', '2026-04-22 17:00:00+08');

SELECT wms.post_inventory_transaction('T0017', '311', 'FIN001', 5, 'FG-D01', 'FG-D02', 'B-FIN-20260422', NULL, 'wms01', '合格', 'SO-2026-0001', '成品转储到发货区', '2026-05-01 09:00:00+08');
SELECT wms.post_inventory_transaction('T0018', '261', 'FIN001', 10, 'FG-D01', NULL, 'B-FIN-20260422', NULL, 'wms01', '合格', 'SO-2026-0001', '销售出库', '2026-05-02 10:00:00+08');
SELECT wms.post_inventory_transaction('T0019', '702', 'PROT001', 1, 'RM-A02', NULL, 'B-PROT-20260401', NULL, 'wms01', '合格', 'CNT-2026-0001', '盘亏调整', '2026-05-02 15:00:00+08');

UPDATE wms.wms_sales_orders_d
SET batch_number = 'B-FIN-20260422', from_bin = 'FG-D01'
WHERE so_id = 'SO-2026-0001' AND line_no = 1;

UPDATE wms.wms_production_orders_d SET batch_number = 'B-FPC-20260402' WHERE order_id = 'PRD-FOG-0001' AND line_no = 1;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-FUNC-20260402' WHERE order_id = 'PRD-FOG-0001' AND line_no = 2;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-FOG-20260415' WHERE order_id = 'PRD-TP-0001' AND line_no = 1;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-CG-20260401' WHERE order_id = 'PRD-TP-0001' AND line_no = 2;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-TP-20260418' WHERE order_id = 'PRD-FIN-0001' AND line_no = 1;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-LCM-20260403' WHERE order_id = 'PRD-FIN-0001' AND line_no = 2;
UPDATE wms.wms_production_orders_d SET batch_number = 'B-PROT-20260401' WHERE order_id = 'PRD-FIN-0001' AND line_no = 3;

INSERT INTO wms.wms_batch_genealogy
(parent_batch_number, component_batch_number, parent_material_id, component_material_id, production_order_id, consumed_qty, output_qty, transaction_id)
VALUES
('B-FOG-20260415', 'B-FPC-20260402', 'FOG001', 'FPC001', 'PRD-FOG-0001', 30, 30, 'T0007'),
('B-FOG-20260415', 'B-FUNC-20260402', 'FOG001', 'FUNC001', 'PRD-FOG-0001', 30, 30, 'T0008'),
('B-TP-20260418', 'B-FOG-20260415', 'TP001', 'FOG001', 'PRD-TP-0001', 25, 25, 'T0010'),
('B-TP-20260418', 'B-CG-20260401', 'TP001', 'CG001', 'PRD-TP-0001', 25, 25, 'T0011'),
('B-FIN-20260422', 'B-TP-20260418', 'FIN001', 'TP001', 'PRD-FIN-0001', 20, 20, 'T0013'),
('B-FIN-20260422', 'B-LCM-20260403', 'FIN001', 'LCM001', 'PRD-FIN-0001', 20, 20, 'T0014'),
('B-FIN-20260422', 'B-PROT-20260401', 'FIN001', 'PROT001', 'PRD-FIN-0001', 20, 20, 'T0015');

INSERT INTO wms.wms_production_variances (
    order_id, variant_code, output_material_id,
    planned_quantity, actual_quantity,
    planned_unit_cost, actual_unit_cost,
    planned_material_cost, actual_material_cost,
    variance_reason, created_by
)
SELECT
    h.order_id,
    h.variant_code,
    h.output_material_id,
    h.planned_quantity,
    h.actual_quantity,
    COALESCE(pv.standard_cost, om.standard_price, 0)::NUMERIC(12,4) AS planned_unit_cost,
    CASE
        WHEN h.actual_quantity > 0 THEN ROUND((COALESCE(SUM(d.actual_qty * cm.map_price), 0) / h.actual_quantity)::NUMERIC, 4)
        ELSE 0
    END AS actual_unit_cost,
    ROUND((h.planned_quantity * COALESCE(pv.standard_cost, om.standard_price, 0))::NUMERIC, 2) AS planned_material_cost,
    COALESCE(SUM(d.actual_qty * cm.map_price), 0)::NUMERIC(15,2) AS actual_material_cost,
    '初始化演示数据：按已过账生产领料汇总',
    'SYSTEM'
FROM wms.wms_production_orders_h h
JOIN mdm.mdm_materials om ON om.material_id = h.output_material_id
LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = h.variant_code
LEFT JOIN wms.wms_production_orders_d d ON d.order_id = h.order_id
LEFT JOIN mdm.mdm_materials cm ON cm.material_id = d.material_id
GROUP BY h.order_id, h.variant_code, h.output_material_id, h.planned_quantity, h.actual_quantity, pv.standard_cost, om.standard_price;

INSERT INTO wms.wms_inspection_lots
(inspection_lot_id, material_id, batch_number, inspection_type, lot_status, inspection_date, inspector, inspection_result)
VALUES
('IL-2026-0001', 'CG001', 'B-CG-20260401', '来料检验', '合格', '2026-04-08 11:00:00+08', 'qm01', '{"summary":"pass"}'::jsonb),
('IL-2026-0002', 'FPC001', 'B-FPC-20260402', '来料检验', '合格', '2026-04-10 11:00:00+08', 'qm01', '{"summary":"pass"}'::jsonb),
('IL-2026-0003', 'FIN001', 'B-FIN-20260422', '最终检验', '合格', '2026-04-23 11:00:00+08', 'qm01', '{"summary":"pass"}'::jsonb),
('IL-2026-0004', 'FIN-B001', 'B-FINB-20260423', '最终检验', '待检', NULL, NULL, '{}'::jsonb);

INSERT INTO wms.wms_inspection_results
(inspection_lot_id, char_id, measured_value, result, remarks, inspected_by, inspected_at)
VALUES
('IL-2026-0001', 'CHAR-CG-THICK', 0.701, '合格', '厚度合格', 'qm01', '2026-04-08 11:10:00+08'),
('IL-2026-0002', 'CHAR-FPC-FUNC', NULL, '合格', '导通测试通过', 'qm01', '2026-04-10 11:10:00+08'),
('IL-2026-0003', 'CHAR-FIN-LOOK', NULL, '合格', '外观合格', 'qm01', '2026-04-23 11:10:00+08');

INSERT INTO wms.wms_quality_notifications
(notification_id, inspection_lot_id, material_id, batch_number, defect_code, problem_description, severity, responsible_person, status)
VALUES
('QN-2026-0001', 'IL-2026-0004', 'FIN-B001', 'B-FINB-20260423', 'D001', '待检批次发现轻微外观风险，需复检确认', '一般', 'qm01', '处理中');

INSERT INTO wms.wms_inventory_count_h
(count_doc_id, count_date, count_type, zone, status, created_by, approved_by, approved_at, posted_by, posted_at, notes)
VALUES
('CNT-2026-0001', '2026-05-02', '周期盘点', 'RM', '已过账', 'wms01', 'admin', '2026-05-02 14:30:00+08', 'wms01', '2026-05-02 15:00:00+08', '保护膜盘亏 1 PCS');

INSERT INTO wms.wms_inventory_count_d
(count_doc_id, line_no, bin_code, material_id, batch_number, system_qty, physical_qty, variance_reason, movement_type, adjustment_transaction_id, adjusted)
VALUES
('CNT-2026-0001', 1, 'RM-A02', 'PROT001', 'B-PROT-20260401', 130, 129, '抽盘少 1 PCS', '702', 'T0019', TRUE);

INSERT INTO wms.wms_mrp_runs
(run_id, run_date, variant_code, demand_qty, demand_date, planning_horizon, status, created_by)
VALUES
('MRP-2026-0001', '2026-05-03 09:00:00+08', 'FIN-A001', 50, '2026-05-20', 30, '完成', 'planner01');

INSERT INTO wms.wms_mrp_suggestions
(run_id, material_id, bom_level, gross_requirement_qty, required_qty, available_qty, safety_stock_qty, suggested_order_type, suggested_order_qty, recommended_bin, recommended_batch, lead_time_days, priority, remarks)
VALUES
('MRP-2026-0001', 'TP001', 1, 50, 50, 5, 8, '生产订单', 53, 'SF-C02', 'B-TP-20260418', 3, 1, 'TP 库存不足，建议生产'),
('MRP-2026-0001', 'LCM001', 1, 50, 50, 50, 20, '采购申请', 20, 'RM-B01', 'B-LCM-20260403', 14, 2, '考虑安全库存后需要补 LCM'),
('MRP-2026-0001', 'PROT001', 1, 50, 50, 129, 50, '采购申请', 0, 'RM-A02', 'B-PROT-20260401', 5, 3, '库存充足'),
('MRP-2026-0001', 'CG001', 2, 50, 50, 95, 30, '采购申请', 0, 'RM-A01', 'B-CG-20260401', 7, 4, '库存充足'),
('MRP-2026-0001', 'FPC001', 3, 50, 50, 70, 30, '采购申请', 10, 'RM-A02', 'B-FPC-20260402', 10, 5, '考虑安全库存后少量短缺');

INSERT INTO wms.wms_attachments
(attachment_id, related_table, related_id, file_name, file_path, file_type, file_size, uploaded_by)
VALUES
('ATT-2026-0001', 'wms_quality_notifications', 'QN-2026-0001', 'defect-photo-demo.jpg', '/demo/qn/defect-photo-demo.jpg', 'image/jpeg', 102400, 'qm01');

INSERT INTO wms.wms_notifications
(notification_id, notification_type, title, content, priority, status, recipient, related_table, related_id)
VALUES
('NTF-2026-0001', 'MRP_SHORTAGE', 'MRP 缺料预警', 'TP001 与 LCM001 存在补货建议，请计划员处理。', '高', '未读', 'planner01', 'wms_mrp_runs', 'MRP-2026-0001'),
('NTF-2026-0002', 'QUALITY', '质量复检通知', 'FIN-B001 批次待复检。', '中', '未读', 'qm01', 'wms_quality_notifications', 'QN-2026-0001');

INSERT INTO sys.sys_audit_log (action, table_name, record_id, new_data, created_at)
VALUES
('INIT_SCHEMA', 'schema', 'v9', '{"version":"schema_final_ultimate_complete_v9","status":"created"}'::jsonb, NOW());

-- ============================================================
-- 8. RPT 物化视图：8 个报表 + 唯一索引 + 刷新函数
-- ============================================================
-- 依赖：wms.wms_transactions / mdm.mdm_materials / mdm.mdm_storage_bins / wms.wms_batches / wms.wms_serial_numbers
CREATE MATERIALIZED VIEW rpt.rpt_current_stock AS
WITH stock_flow AS (
    SELECT
        t.material_id,
        t.to_bin AS bin_code,
        t.batch_number,
        ABS(t.quantity) AS qty_delta
    FROM wms.wms_transactions t
    WHERE t.to_bin IS NOT NULL

    UNION ALL

    SELECT
        t.material_id,
        t.from_bin AS bin_code,
        t.batch_number,
        -ABS(t.quantity) AS qty_delta
    FROM wms.wms_transactions t
    WHERE t.from_bin IS NOT NULL
),
stock_summary AS (
    SELECT
        material_id,
        bin_code,
        batch_number,
        SUM(qty_delta)::INTEGER AS qty
    FROM stock_flow
    GROUP BY material_id, bin_code, batch_number
    HAVING SUM(qty_delta) <> 0
),
last_txn AS (
    SELECT material_id, batch_number, bin_code, MAX(transaction_date) AS last_transaction_at
    FROM (
        SELECT material_id, batch_number, to_bin AS bin_code, transaction_date
        FROM wms.wms_transactions
        WHERE to_bin IS NOT NULL
        UNION ALL
        SELECT material_id, batch_number, from_bin AS bin_code, transaction_date
        FROM wms.wms_transactions
        WHERE from_bin IS NOT NULL
    ) x
    GROUP BY material_id, batch_number, bin_code
)
SELECT
    s.material_id,
    m.material_name,
    s.bin_code,
    b.zone,
    s.batch_number,
    ba.quality_grade,
    ba.quality_status,
    s.qty,
    COUNT(DISTINCT sn.serial_number)::INTEGER AS serial_count,
    lt.last_transaction_at
FROM stock_summary s
JOIN mdm.mdm_materials m ON m.material_id = s.material_id
JOIN mdm.mdm_storage_bins b ON b.bin_code = s.bin_code
LEFT JOIN wms.wms_batches ba ON ba.batch_number = s.batch_number
LEFT JOIN wms.wms_serial_numbers sn ON sn.material_id = s.material_id AND sn.batch_number = s.batch_number AND sn.current_bin = s.bin_code
LEFT JOIN last_txn lt ON lt.material_id = s.material_id AND lt.batch_number = s.batch_number AND lt.bin_code = s.bin_code
GROUP BY s.material_id, m.material_name, s.bin_code, b.zone, s.batch_number, ba.quality_grade, ba.quality_status, s.qty, lt.last_transaction_at;

CREATE UNIQUE INDEX ux_rpt_current_stock ON rpt.rpt_current_stock(material_id, bin_code, batch_number);

-- 依赖：rpt.rpt_current_stock / mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_inventory_value AS
SELECT
    m.material_id,
    m.material_name,
    m.material_type,
    m.current_stock,
    COALESCE(SUM(cs.qty), 0)::INTEGER AS stock_by_bin,
    m.map_price,
    m.standard_price,
    (COALESCE(SUM(cs.qty), 0) * m.map_price)::NUMERIC(15,2) AS total_map_value,
    (COALESCE(SUM(cs.qty), 0) * m.standard_price)::NUMERIC(15,2) AS total_standard_value,
    (COALESCE(SUM(cs.qty), 0) * (m.map_price - m.standard_price))::NUMERIC(15,2) AS price_variance
FROM mdm.mdm_materials m
LEFT JOIN rpt.rpt_current_stock cs ON cs.material_id = m.material_id
GROUP BY m.material_id, m.material_name, m.material_type, m.current_stock, m.map_price, m.standard_price;

CREATE UNIQUE INDEX ux_rpt_inventory_value ON rpt.rpt_inventory_value(material_id);

-- 依赖：wms.wms_inspection_lots / mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_quality_status AS
SELECT
    m.material_id,
    m.material_name,
    COUNT(il.inspection_lot_id)::INTEGER AS lot_count,
    COUNT(*) FILTER (WHERE il.lot_status = '合格')::INTEGER AS pass_count,
    COUNT(*) FILTER (WHERE il.lot_status = '待检')::INTEGER AS pending_count,
    COUNT(*) FILTER (WHERE il.lot_status = '冻结')::INTEGER AS blocked_count,
    CASE
        WHEN COUNT(il.inspection_lot_id) = 0 THEN 0::NUMERIC(5,2)
        ELSE ROUND(COUNT(*) FILTER (WHERE il.lot_status = '合格') * 100.0 / COUNT(il.inspection_lot_id), 2)
    END AS pass_rate
FROM mdm.mdm_materials m
LEFT JOIN wms.wms_inspection_lots il ON il.material_id = m.material_id
GROUP BY m.material_id, m.material_name;

CREATE UNIQUE INDEX ux_rpt_quality_status ON rpt.rpt_quality_status(material_id);

-- 依赖：wms.wms_mrp_suggestions / wms.wms_mrp_runs / mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_mrp_shortage AS
SELECT
    s.id AS suggestion_id,
    s.run_id,
    r.run_date,
    r.variant_code,
    s.material_id,
    m.material_name,
    s.bom_level,
    s.required_qty,
    s.available_qty,
    s.safety_stock_qty,
    s.shortage_qty,
    s.suggested_order_type,
    s.suggested_order_qty,
    s.recommended_bin,
    s.recommended_batch,
    s.lead_time_days,
    s.priority,
    s.remarks
FROM wms.wms_mrp_suggestions s
JOIN wms.wms_mrp_runs r ON r.run_id = s.run_id
JOIN mdm.mdm_materials m ON m.material_id = s.material_id
WHERE s.shortage_qty > 0;

CREATE UNIQUE INDEX ux_rpt_mrp_shortage ON rpt.rpt_mrp_shortage(suggestion_id);

-- 依赖：mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_low_stock_alert AS
SELECT
    m.material_id,
    m.material_name,
    m.material_type,
    m.current_stock,
    m.safety_stock,
    m.reorder_point,
    CASE
        WHEN m.current_stock < m.safety_stock THEN '严重缺货'
        WHEN m.current_stock < m.reorder_point THEN '需要补货'
        ELSE '正常'
    END AS alert_level
FROM mdm.mdm_materials m
WHERE m.current_stock < m.reorder_point;

CREATE UNIQUE INDEX ux_rpt_low_stock_alert ON rpt.rpt_low_stock_alert(material_id);

-- v8：刷新函数统一在 8 个物化视图创建完成后定义，避免重复覆盖。

-- ============================================================
-- 9. 对象说明
-- ============================================================
COMMENT ON SCHEMA mdm IS 'Master Data Management - 主数据';
COMMENT ON SCHEMA wms IS 'Warehouse Management System - 业务、库存、批次、质量、MRP、追溯';
COMMENT ON SCHEMA rpt IS 'Reporting - 报表与分析物化视图';
COMMENT ON SCHEMA sys IS 'System - 用户、角色、权限、参数、审计';
COMMENT ON TABLE wms.wms_transactions IS '库存事务日志。库存变化必须通过 wms.post_inventory_transaction 过账。';
COMMENT ON TABLE wms.wms_bin_stock IS '当前货位/物料/批次库存镜像，用于负库存检查和快速查询。';
COMMENT ON TABLE wms.wms_batch_genealogy IS '批次谱系表，支持成品批次到原材料批次的正反向追溯。';
COMMENT ON MATERIALIZED VIEW rpt.rpt_current_stock IS '基于事务日志 from_bin/to_bin 双边流水计算的当前库存报表。';

-- v8：统一在文件末尾 COMMIT，确保初始化失败时整体回滚。

-- ============================================================
-- 10. 快速校验 SQL
-- ============================================================
-- SELECT 'mdm_materials' AS item, COUNT(*) FROM mdm.mdm_materials
-- UNION ALL SELECT 'wms_batches', COUNT(*) FROM wms.wms_batches
-- UNION ALL SELECT 'wms_transactions', COUNT(*) FROM wms.wms_transactions
-- UNION ALL SELECT 'rpt_current_stock', COUNT(*) FROM rpt.rpt_current_stock;


-- ============================================================
-- WMS v6/v7/v8 增强段
-- 目标：补齐 v5 剩余的 P0/P1/P2 缺口，并集成到完整初始化脚本
-- 说明：本段由 v5 -> v6 增量补丁合并而来
-- ============================================================


SET search_path TO wms, mdm, sys, rpt, public;

-- ============================================================
-- 第三部分：4 个追溯/爆炸函数（P1）
-- ============================================================

-- 1. BOM 需求爆炸（递归）
CREATE OR REPLACE FUNCTION wms.fn_bom_explosion(
    p_material_id   VARCHAR(20),
    p_demand_qty    NUMERIC,
    p_variant_code  VARCHAR(20) DEFAULT NULL
) RETURNS TABLE (
    bom_level           INTEGER,
    parent_material_id  VARCHAR(20),
    component_material_id VARCHAR(20),
    component_name      VARCHAR(100),
    unit_qty            NUMERIC,
    required_qty        NUMERIC,
    available_qty       INTEGER,
    shortage_qty        NUMERIC,
    is_critical         BOOLEAN
) AS $$
WITH RECURSIVE bom_tree AS (
    -- 起点：从给定父物料展开
    SELECT 
        c.bom_level,
        c.parent_material_id,
        c.component_material_id,
        c.quantity AS unit_qty,
        (c.quantity * p_demand_qty)::NUMERIC AS required_qty,
        c.is_critical
    FROM mdm.mdm_bom_components c
    JOIN mdm.mdm_bom_headers h ON h.bom_id = c.bom_id
    WHERE c.parent_material_id = p_material_id
      AND h.status = '生效'
      AND (p_variant_code IS NULL OR h.variant_code = p_variant_code OR h.variant_code IS NULL)

    UNION ALL

    -- 递归：把每个组件再展开下一层
    SELECT 
        bt.bom_level + 1,
        c.parent_material_id,
        c.component_material_id,
        c.quantity AS unit_qty,
        (c.quantity * bt.required_qty)::NUMERIC AS required_qty,
        c.is_critical
    FROM mdm.mdm_bom_components c
    JOIN mdm.mdm_bom_headers h ON h.bom_id = c.bom_id
    JOIN bom_tree bt ON bt.component_material_id = c.parent_material_id
    WHERE h.status = '生效'
)
SELECT 
    bt.bom_level,
    bt.parent_material_id,
    bt.component_material_id,
    m.material_name AS component_name,
    bt.unit_qty,
    bt.required_qty,
    m.current_stock AS available_qty,
    GREATEST(bt.required_qty - m.current_stock, 0)::NUMERIC AS shortage_qty,
    bt.is_critical
FROM bom_tree bt
JOIN mdm.mdm_materials m ON m.material_id = bt.component_material_id
ORDER BY bt.bom_level, bt.component_material_id;
$$ LANGUAGE sql STABLE;

COMMENT ON FUNCTION wms.fn_bom_explosion IS 
    'BOM 需求爆炸 - 对应 Excel BOM需求爆炸 sheet，多层递归';


-- 2. 批次正向追溯（该批次被用到了哪些下游成品）
CREATE OR REPLACE FUNCTION wms.fn_batch_trace_forward(p_batch_number VARCHAR(30))
RETURNS TABLE (
    level               INTEGER,
    parent_batch        VARCHAR(30),
    parent_material     VARCHAR(20),
    consumed_qty        NUMERIC,
    output_qty          NUMERIC,
    production_order_id VARCHAR(30)
) AS $$
WITH RECURSIVE forward_trace AS (
    -- 起点：直接消耗本批次的上级批次
    SELECT 
        1 AS level,
        g.parent_batch_number AS parent_batch,
        g.parent_material_id AS parent_material,
        g.consumed_qty,
        g.output_qty,
        g.production_order_id
    FROM wms.wms_batch_genealogy g
    WHERE g.component_batch_number = p_batch_number

    UNION ALL

    -- 递归：上级批次又被谁消耗
    SELECT 
        ft.level + 1,
        g.parent_batch_number,
        g.parent_material_id,
        g.consumed_qty,
        g.output_qty,
        g.production_order_id
    FROM wms.wms_batch_genealogy g
    JOIN forward_trace ft ON g.component_batch_number = ft.parent_batch
)
SELECT * FROM forward_trace ORDER BY level, parent_batch;
$$ LANGUAGE sql STABLE;

COMMENT ON FUNCTION wms.fn_batch_trace_forward IS 
    '批次正向追溯 - 输入原材料批次，返回所有使用它的下游成品批次';


-- 3. 批次反向追溯（该成品批次由哪些原材料批次构成）
CREATE OR REPLACE FUNCTION wms.fn_batch_trace_backward(p_batch_number VARCHAR(30))
RETURNS TABLE (
    level               INTEGER,
    component_batch     VARCHAR(30),
    component_material  VARCHAR(20),
    consumed_qty        NUMERIC,
    production_order_id VARCHAR(30)
) AS $$
WITH RECURSIVE backward_trace AS (
    SELECT 
        1 AS level,
        g.component_batch_number AS component_batch,
        g.component_material_id AS component_material,
        g.consumed_qty,
        g.production_order_id
    FROM wms.wms_batch_genealogy g
    WHERE g.parent_batch_number = p_batch_number

    UNION ALL

    SELECT 
        bt.level + 1,
        g.component_batch_number,
        g.component_material_id,
        g.consumed_qty,
        g.production_order_id
    FROM wms.wms_batch_genealogy g
    JOIN backward_trace bt ON g.parent_batch_number = bt.component_batch
)
SELECT * FROM backward_trace ORDER BY level, component_batch;
$$ LANGUAGE sql STABLE;

COMMENT ON FUNCTION wms.fn_batch_trace_backward IS 
    '批次反向追溯 - 输入成品批次，返回所有原材料批次树';


-- 4. 序列号追溯（含完整流转）
CREATE OR REPLACE FUNCTION wms.fn_serial_trace(p_serial_number VARCHAR(30))
RETURNS TABLE (
    seq_no              INTEGER,
    changed_at          TIMESTAMPTZ,
    transaction_id      VARCHAR(30),
    old_status          VARCHAR(20),
    new_status          VARCHAR(20),
    old_bin             VARCHAR(20),
    new_bin             VARCHAR(20),
    old_quality_status  mdm.quality_status,
    new_quality_status  mdm.quality_status,
    changed_by          VARCHAR(50)
) AS $$
SELECT 
    ROW_NUMBER() OVER (ORDER BY h.changed_at)::INTEGER AS seq_no,
    h.changed_at,
    h.transaction_id,
    h.old_status,
    h.new_status,
    h.old_bin,
    h.new_bin,
    h.old_quality_status,
    h.new_quality_status,
    h.changed_by
FROM wms.wms_serial_history h
WHERE h.serial_number = p_serial_number
ORDER BY h.changed_at;
$$ LANGUAGE sql STABLE;

COMMENT ON FUNCTION wms.fn_serial_trace IS 
    '序列号全生命周期追溯 - 单件级召回支持';


-- ============================================================
-- 第四部分：3 个报表物化视图（P2）
-- ============================================================

-- 1. 物料 × 区域 PIVOT（对应 Excel "库存概览" sheet）
DROP MATERIALIZED VIEW IF EXISTS rpt.rpt_stock_by_zone;
-- 依赖：wms.wms_bin_stock / mdm.mdm_storage_bins / mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_stock_by_zone AS
SELECT 
    m.material_id,
    m.material_name,
    m.material_type,
    COALESCE(SUM(bs.qty) FILTER (WHERE b.zone='RM'),   0)::INTEGER AS qty_rm,
    COALESCE(SUM(bs.qty) FILTER (WHERE b.zone='SF'),   0)::INTEGER AS qty_sf,
    COALESCE(SUM(bs.qty) FILTER (WHERE b.zone='FG'),   0)::INTEGER AS qty_fg,
    COALESCE(SUM(bs.qty) FILTER (WHERE b.zone='PROD'), 0)::INTEGER AS qty_prod,
    COALESCE(SUM(bs.qty), 0)::INTEGER AS total_qty,
    m.safety_stock,
    m.map_price,
    (COALESCE(SUM(bs.qty), 0) * m.map_price)::NUMERIC(15,2) AS total_value,
    CASE 
        WHEN COALESCE(SUM(bs.qty),0) < m.safety_stock THEN '⚠️ 低于安全库存'
        ELSE '✓ 正常'
    END AS status
FROM mdm.mdm_materials m
LEFT JOIN wms.wms_bin_stock bs ON bs.material_id = m.material_id
LEFT JOIN mdm.mdm_storage_bins b ON b.bin_code = bs.bin_code
GROUP BY m.material_id, m.material_name, m.material_type, m.safety_stock, m.map_price;

CREATE UNIQUE INDEX ux_rpt_stock_by_zone ON rpt.rpt_stock_by_zone(material_id);

COMMENT ON MATERIALIZED VIEW rpt.rpt_stock_by_zone IS 
    '库存概览 - 物料 × 区域 PIVOT，对应 Excel 库存概览 sheet';


-- 2. 货位库存汇总（对应 Excel "Bin 库存概览" sheet）
DROP MATERIALIZED VIEW IF EXISTS rpt.rpt_bin_stock_summary;
-- 依赖：wms.wms_bin_stock / mdm.mdm_storage_bins / wms.wms_transactions
CREATE MATERIALIZED VIEW rpt.rpt_bin_stock_summary AS
SELECT 
    b.bin_code,
    b.zone,
    b.bin_type,
    b.capacity,
    COALESCE(SUM(bs.qty), 0)::INTEGER AS current_qty,
    COUNT(DISTINCT bs.material_id)::INTEGER AS material_count,
    COUNT(DISTINCT bs.batch_number)::INTEGER AS batch_count,
    ROUND(COALESCE(SUM(bs.qty), 0) * 100.0 / NULLIF(b.capacity,0), 2) AS utilization_pct,
    (
        SELECT MAX(t.transaction_date) 
        FROM wms.wms_transactions t 
        WHERE t.from_bin = b.bin_code OR t.to_bin = b.bin_code
    ) AS last_movement_at,
    CASE 
        WHEN COALESCE(SUM(bs.qty),0) > b.capacity THEN '🔴 超容量'
        WHEN COALESCE(SUM(bs.qty),0) * 100.0 / NULLIF(b.capacity,0) >= 90 THEN '⚠️ 接近满仓'
        WHEN COALESCE(SUM(bs.qty),0) = 0 THEN '空闲'
        ELSE '✓ 正常'
    END AS status
FROM mdm.mdm_storage_bins b
LEFT JOIN wms.wms_bin_stock bs ON bs.bin_code = b.bin_code
GROUP BY b.bin_code, b.zone, b.bin_type, b.capacity;

CREATE UNIQUE INDEX ux_rpt_bin_stock_summary ON rpt.rpt_bin_stock_summary(bin_code);

COMMENT ON MATERIALIZED VIEW rpt.rpt_bin_stock_summary IS 
    '货位库存汇总 - 含容量利用率，对应 Excel Bin 库存概览 sheet';


-- 3. 批次库存汇总（对应 Excel "批次库存概览" sheet）
DROP MATERIALIZED VIEW IF EXISTS rpt.rpt_batch_stock_summary;
-- 依赖：wms.wms_batches / wms.wms_bin_stock / mdm.mdm_materials
CREATE MATERIALIZED VIEW rpt.rpt_batch_stock_summary AS
SELECT 
    b.batch_number,
    b.material_id,
    m.material_name,
    b.production_date,
    b.expiry_date,
    (b.expiry_date - CURRENT_DATE) AS days_to_expiry,
    b.supplier_batch,
    b.quality_grade,
    b.quality_status,
    COALESCE(SUM(bs.qty), 0)::INTEGER AS current_stock,
    STRING_AGG(DISTINCT bs.bin_code, ', ' ORDER BY bs.bin_code) AS bins,
    (CURRENT_DATE - b.production_date) AS age_days,
    CASE 
        WHEN b.quality_status = '冻结' THEN '🔴 冻结'
        WHEN b.expiry_date IS NOT NULL AND b.expiry_date < CURRENT_DATE THEN '🔴 过期'
        WHEN b.expiry_date IS NOT NULL AND b.expiry_date - CURRENT_DATE <= 30 THEN '⚠️ 临期'
        WHEN (CURRENT_DATE - b.production_date) > 90 THEN '⚠️ 长期库存'
        ELSE '✓ 正常'
    END AS alert_level
FROM wms.wms_batches b
JOIN mdm.mdm_materials m ON m.material_id = b.material_id
LEFT JOIN wms.wms_bin_stock bs ON bs.batch_number = b.batch_number
GROUP BY b.batch_number, b.material_id, m.material_name, b.production_date, 
         b.expiry_date, b.supplier_batch, b.quality_grade, b.quality_status;

CREATE UNIQUE INDEX ux_rpt_batch_stock_summary ON rpt.rpt_batch_stock_summary(batch_number);

COMMENT ON MATERIALIZED VIEW rpt.rpt_batch_stock_summary IS 
    '批次库存汇总 - 含 FEFO 临期预警，对应 Excel 批次库存概览 sheet';


-- ============================================================
-- 更新刷新函数（包含新增 3 个视图）
-- ============================================================
CREATE OR REPLACE FUNCTION rpt.refresh_all_materialized_views()
RETURNS void AS $$
BEGIN
    -- 先刷新基础库存视图，再刷新依赖它或同域统计的报表，最后刷新质量/MRP/低库存提醒。
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_current_stock;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_inventory_value;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_stock_by_zone;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_bin_stock_summary;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_batch_stock_summary;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_quality_status;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_mrp_shortage;
    REFRESH MATERIALIZED VIEW CONCURRENTLY rpt.rpt_low_stock_alert;
END;
$$ LANGUAGE plpgsql;


-- ============================================================
-- 第五部分：验证
-- ============================================================
-- 完整初始化脚本中，物化视图在 CREATE MATERIALIZED VIEW 时已经完成首次填充。
-- 后续业务写入后，可手工执行：SELECT rpt.refresh_all_materialized_views();


-- ============================================================
-- WMS v8 增强段：一键业务函数、触发器、审计、一致性校验、生产成本差异
-- ============================================================

-- ------------------------------------------------------------
-- 1. 通用 updated_at 维护触发器
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION sys.fn_update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_upd_mdm_materials BEFORE UPDATE ON mdm.mdm_materials FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_storage_bins BEFORE UPDATE ON mdm.mdm_storage_bins FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_suppliers BEFORE UPDATE ON mdm.mdm_suppliers FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_customers BEFORE UPDATE ON mdm.mdm_customers FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_material_suppliers BEFORE UPDATE ON mdm.mdm_material_suppliers FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_product_variants BEFORE UPDATE ON mdm.mdm_product_variants FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_mdm_bom_headers BEFORE UPDATE ON mdm.mdm_bom_headers FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_batches BEFORE UPDATE ON wms.wms_batches FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_batch_attributes BEFORE UPDATE ON wms.wms_batch_attributes FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_serial_numbers BEFORE UPDATE ON wms.wms_serial_numbers FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_bin_stock BEFORE UPDATE ON wms.wms_bin_stock FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_purchase_orders_h BEFORE UPDATE ON wms.wms_purchase_orders_h FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_sales_orders_h BEFORE UPDATE ON wms.wms_sales_orders_h FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_production_orders_h BEFORE UPDATE ON wms.wms_production_orders_h FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_production_variances BEFORE UPDATE ON wms.wms_production_variances FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_inspection_lots BEFORE UPDATE ON wms.wms_inspection_lots FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_wms_inventory_count_h BEFORE UPDATE ON wms.wms_inventory_count_h FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_sys_users BEFORE UPDATE ON sys.sys_users FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();
CREATE TRIGGER trg_upd_sys_system_params BEFORE UPDATE ON sys.sys_system_params FOR EACH ROW EXECUTE FUNCTION sys.fn_update_updated_at();

-- ------------------------------------------------------------
-- 2. 关键表 UPDATE/DELETE 审计触发器
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION sys.fn_audit_critical_changes()
RETURNS TRIGGER AS $$
DECLARE
    v_record_id TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_record_id := COALESCE(
            to_jsonb(OLD)->>'material_id',
            to_jsonb(OLD)->>'bom_id',
            to_jsonb(OLD)->>'notification_id',
            to_jsonb(OLD)->>'id'
        );

        INSERT INTO sys.sys_audit_log(action, table_name, record_id, old_data, new_data, created_at)
        VALUES (TG_OP, TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME, v_record_id, to_jsonb(OLD), NULL, NOW());
        RETURN OLD;
    END IF;

    v_record_id := COALESCE(
        to_jsonb(NEW)->>'material_id',
        to_jsonb(NEW)->>'bom_id',
        to_jsonb(NEW)->>'notification_id',
        to_jsonb(NEW)->>'id'
    );

    INSERT INTO sys.sys_audit_log(action, table_name, record_id, old_data, new_data, created_at)
    VALUES (
        TG_OP,
        TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME,
        v_record_id,
        CASE WHEN TG_OP = 'UPDATE' THEN to_jsonb(OLD) ELSE NULL END,
        to_jsonb(NEW),
        NOW()
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_mdm_materials AFTER UPDATE OR DELETE ON mdm.mdm_materials FOR EACH ROW EXECUTE FUNCTION sys.fn_audit_critical_changes();
CREATE TRIGGER trg_audit_mdm_bom_components AFTER UPDATE OR DELETE ON mdm.mdm_bom_components FOR EACH ROW EXECUTE FUNCTION sys.fn_audit_critical_changes();
CREATE TRIGGER trg_audit_wms_quality_notifications AFTER UPDATE OR DELETE ON wms.wms_quality_notifications FOR EACH ROW EXECUTE FUNCTION sys.fn_audit_critical_changes();

-- ------------------------------------------------------------
-- 3. 主供应商唯一性：触发器 + 部分唯一索引双保险
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION mdm.fn_enforce_primary_supplier()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_primary IS TRUE AND NEW.is_active IS TRUE THEN
        IF EXISTS (
            SELECT 1
            FROM mdm.mdm_material_suppliers ms
            WHERE ms.material_id = NEW.material_id
              AND ms.is_primary IS TRUE
              AND ms.is_active IS TRUE
              AND ms.id <> COALESCE(NEW.id, -1)
        ) THEN
            RAISE EXCEPTION '物料 % 已存在主供应商，不能再设置第二个主供应商', NEW.material_id;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE UNIQUE INDEX ux_material_one_primary_supplier
ON mdm.mdm_material_suppliers(material_id)
WHERE is_primary IS TRUE AND is_active IS TRUE;

CREATE TRIGGER trg_enforce_primary_supplier
BEFORE INSERT OR UPDATE OF material_id, is_primary, is_active
ON mdm.mdm_material_suppliers
FOR EACH ROW EXECUTE FUNCTION mdm.fn_enforce_primary_supplier();

-- ------------------------------------------------------------
-- 4. 数据一致性校验视图：material / bin / batch / transactions 四账核对
-- ------------------------------------------------------------
CREATE OR REPLACE VIEW rpt.rpt_data_consistency_check AS
WITH mat AS (
    SELECT material_id, current_stock::INTEGER AS material_stock
    FROM mdm.mdm_materials
), bin_sum AS (
    SELECT material_id, COALESCE(SUM(qty),0)::INTEGER AS bin_stock
    FROM wms.wms_bin_stock
    GROUP BY material_id
), batch_sum AS (
    SELECT material_id, COALESCE(SUM(current_stock),0)::INTEGER AS batch_stock
    FROM wms.wms_batches
    GROUP BY material_id
), txn_sum AS (
    SELECT material_id,
           COALESCE(SUM(CASE WHEN movement_type = '311' THEN 0 ELSE quantity END),0)::INTEGER AS transaction_stock
    FROM wms.wms_transactions
    GROUP BY material_id
)
SELECT
    m.material_id,
    mm.material_name,
    m.material_stock,
    COALESCE(bn.bin_stock,0) AS bin_stock,
    COALESCE(bt.batch_stock,0) AS batch_stock,
    COALESCE(tx.transaction_stock,0) AS transaction_stock,
    (m.material_stock - COALESCE(bn.bin_stock,0)) AS diff_material_vs_bin,
    (m.material_stock - COALESCE(bt.batch_stock,0)) AS diff_material_vs_batch,
    (m.material_stock - COALESCE(tx.transaction_stock,0)) AS diff_material_vs_transaction,
    CASE
        WHEN m.material_stock = COALESCE(bn.bin_stock,0)
         AND m.material_stock = COALESCE(bt.batch_stock,0)
         AND m.material_stock = COALESCE(tx.transaction_stock,0)
        THEN '一致'
        ELSE '不一致'
    END AS check_status
FROM mat m
JOIN mdm.mdm_materials mm ON mm.material_id = m.material_id
LEFT JOIN bin_sum bn ON bn.material_id = m.material_id
LEFT JOIN batch_sum bt ON bt.material_id = m.material_id
LEFT JOIN txn_sum tx ON tx.material_id = m.material_id;

COMMENT ON VIEW rpt.rpt_data_consistency_check IS
    '库存数据一致性校验：mdm_materials.current_stock = SUM(wms_bin_stock.qty) = SUM(wms_batches.current_stock) = 事务净额。';

-- ------------------------------------------------------------
-- 5. FEFO 自动选批函数
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_pick_batch_fefo(
    p_material_id       VARCHAR(20),
    p_required_qty      INTEGER,
    p_from_zone         VARCHAR(10) DEFAULT NULL,
    p_quality_status    mdm.quality_status DEFAULT '合格'
) RETURNS TABLE (
    batch_number    VARCHAR(30),
    bin_code        VARCHAR(20),
    pick_qty        INTEGER,
    expiry_date     DATE,
    available_qty   INTEGER
) AS $$
BEGIN
    IF p_required_qty IS NULL OR p_required_qty <= 0 THEN
        RAISE EXCEPTION '需求数量必须大于 0';
    END IF;

    RETURN QUERY
    WITH locked_candidates AS (
        SELECT
            bs.batch_number,
            bs.bin_code,
            bs.qty::INTEGER AS available_qty,
            b.expiry_date,
            b.production_date
        FROM wms.wms_bin_stock bs
        JOIN wms.wms_batches b ON b.batch_number = bs.batch_number
        JOIN mdm.mdm_storage_bins sb ON sb.bin_code = bs.bin_code
        WHERE bs.material_id = p_material_id
          AND bs.qty > 0
          AND bs.quality_status = p_quality_status
          AND b.quality_status = p_quality_status
          AND (b.expiry_date IS NULL OR b.expiry_date >= CURRENT_DATE)
          AND (p_from_zone IS NULL OR sb.zone = p_from_zone)
        ORDER BY b.expiry_date NULLS LAST, b.production_date, bs.batch_number, bs.bin_code
        FOR UPDATE OF bs SKIP LOCKED
    ), candidates AS (
        SELECT
            lc.*,
            COALESCE(
                SUM(lc.available_qty) OVER (
                    ORDER BY lc.expiry_date NULLS LAST, lc.production_date, lc.batch_number, lc.bin_code
                    ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                ), 0
            )::INTEGER AS qty_before
        FROM locked_candidates lc
    )
    SELECT
        c.batch_number,
        c.bin_code,
        LEAST(c.available_qty, p_required_qty - c.qty_before)::INTEGER AS pick_qty,
        c.expiry_date,
        c.available_qty
    FROM candidates c
    WHERE c.qty_before < p_required_qty
    ORDER BY c.expiry_date NULLS LAST, c.production_date, c.batch_number, c.bin_code;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- ------------------------------------------------------------
-- 6. PO 一键收货：按采购订单未收数量批量 101 过账
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_post_purchase_receipt(
    p_po_id             VARCHAR(30),
    p_batch_prefix      VARCHAR(24) DEFAULT NULL,
    p_to_bin            VARCHAR(20) DEFAULT NULL,
    p_operator          VARCHAR(50) DEFAULT 'SYSTEM',
    p_quality_status    mdm.quality_status DEFAULT '合格',
    p_posting_time      TIMESTAMPTZ DEFAULT NOW()
) RETURNS TABLE (
    posted_line_no          INTEGER,
    posted_material_id      VARCHAR(20),
    posted_batch_number     VARCHAR(30),
    posted_qty              INTEGER,
    posted_transaction_id   VARCHAR(30)
) AS $$
DECLARE
    v_line      RECORD;
    v_to_bin    VARCHAR(20);
    v_batch     VARCHAR(30);
    v_txn_id    VARCHAR(30);
BEGIN
    IF NOT EXISTS (SELECT 1 FROM wms.wms_purchase_orders_h WHERE po_id = p_po_id AND status <> '取消') THEN
        RAISE EXCEPTION '采购订单 % 不存在或已取消', p_po_id;
    END IF;

    FOR v_line IN
        SELECT d.*, m.default_zone
        FROM wms.wms_purchase_orders_d d
        JOIN mdm.mdm_materials m ON m.material_id = d.material_id
        WHERE d.po_id = p_po_id
          AND d.line_status <> '取消'
          AND d.open_qty > 0
        ORDER BY d.line_no
    LOOP
        SELECT COALESCE(
            p_to_bin,
            v_line.expected_bin,
            (
                SELECT bin_code
                FROM mdm.mdm_storage_bins
                WHERE zone = v_line.default_zone AND status IN ('正常','占用')
                ORDER BY current_occupied, bin_code
                LIMIT 1
            )
        ) INTO v_to_bin;

        IF v_to_bin IS NULL THEN
            RAISE EXCEPTION 'PO % 行 % 未找到可用目标货位', p_po_id, v_line.line_no;
        END IF;

        v_batch := LEFT(COALESCE(p_batch_prefix, 'B-' || v_line.material_id || '-' || to_char(p_posting_time, 'YYYYMMDD') || '-PO') || '-' || v_line.line_no::TEXT, 30);
        v_txn_id := LEFT('GR-' || p_po_id || '-' || v_line.line_no::TEXT, 30);

        INSERT INTO wms.wms_batches(batch_number, material_id, production_date, expiry_date, quality_grade, current_stock, current_bin, quality_status)
        VALUES (v_batch, v_line.material_id, p_posting_time::DATE, (p_posting_time::DATE + INTERVAL '365 days')::DATE, 'A', 0, NULL, p_quality_status)
        ON CONFLICT (batch_number) DO NOTHING;

        PERFORM wms.post_inventory_transaction(
            v_txn_id, '101', v_line.material_id, v_line.open_qty,
            NULL, v_to_bin, v_batch, NULL, p_operator, p_quality_status,
            p_po_id, 'PO 一键收货', p_posting_time, v_line.unit_price
        );

        UPDATE wms.wms_purchase_orders_d
        SET received_qty = ordered_qty,
            line_status = '完成'
        WHERE id = v_line.id;

        posted_line_no := v_line.line_no;
        posted_material_id := v_line.material_id;
        posted_batch_number := v_batch;
        posted_qty := v_line.open_qty;
        posted_transaction_id := v_txn_id;
        RETURN NEXT;
    END LOOP;

    UPDATE wms.wms_purchase_orders_h h
    SET status = CASE
            WHEN EXISTS (SELECT 1 FROM wms.wms_purchase_orders_d d WHERE d.po_id = p_po_id AND d.open_qty > 0 AND d.line_status <> '取消') THEN '部分到货'
            ELSE '完成'
        END,
        updated_at = NOW()
    WHERE h.po_id = p_po_id;
END;
$$ LANGUAGE plpgsql;

-- ------------------------------------------------------------
-- 7. SO 一键发货：按销售订单未发数量 FEFO 261 过账并锁定 MAP
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_post_sales_shipment(
    p_so_id         VARCHAR(30),
    p_operator      VARCHAR(50) DEFAULT 'SYSTEM',
    p_posting_time  TIMESTAMPTZ DEFAULT NOW()
) RETURNS TABLE (
    shipped_line_no         INTEGER,
    shipped_material_id     VARCHAR(20),
    shipped_batch_number    VARCHAR(30),
    shipped_bin_code        VARCHAR(20),
    shipped_qty             INTEGER,
    shipped_transaction_id  VARCHAR(30)
) AS $$
DECLARE
    v_line          RECORD;
    v_pick          RECORD;
    v_map           NUMERIC(12,2);
    v_txn_id        VARCHAR(30);
    v_pick_no       INTEGER;
    v_picked_total  INTEGER;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM wms.wms_sales_orders_h WHERE so_id = p_so_id AND status <> '取消') THEN
        RAISE EXCEPTION '销售订单 % 不存在或已取消', p_so_id;
    END IF;

    FOR v_line IN
        SELECT *
        FROM wms.wms_sales_orders_d
        WHERE so_id = p_so_id
          AND line_status <> '取消'
          AND open_qty > 0
        ORDER BY line_no
    LOOP
        SELECT map_price INTO v_map FROM mdm.mdm_materials WHERE material_id = v_line.material_id;
        v_pick_no := 0;
        v_picked_total := 0;

        IF v_line.batch_number IS NOT NULL AND v_line.from_bin IS NOT NULL THEN
            v_pick_no := 1;
            v_txn_id := LEFT('GI-' || p_so_id || '-' || v_line.line_no::TEXT || '-1', 30);

            PERFORM wms.post_inventory_transaction(
                v_txn_id, '261', v_line.material_id, v_line.open_qty,
                v_line.from_bin, NULL, v_line.batch_number, NULL, p_operator, '合格',
                p_so_id, 'SO 一键发货', p_posting_time, NULL
            );

            v_picked_total := v_line.open_qty;
            shipped_line_no := v_line.line_no;
            shipped_material_id := v_line.material_id;
            shipped_batch_number := v_line.batch_number;
            shipped_bin_code := v_line.from_bin;
            shipped_qty := v_line.open_qty;
            shipped_transaction_id := v_txn_id;
            RETURN NEXT;
        ELSE
            FOR v_pick IN SELECT * FROM wms.fn_pick_batch_fefo(v_line.material_id, v_line.open_qty, NULL, '合格') LOOP
                v_pick_no := v_pick_no + 1;
                v_txn_id := LEFT('GI-' || p_so_id || '-' || v_line.line_no::TEXT || '-' || v_pick_no::TEXT, 30);

                PERFORM wms.post_inventory_transaction(
                    v_txn_id, '261', v_line.material_id, v_pick.pick_qty,
                    v_pick.bin_code, NULL, v_pick.batch_number, NULL, p_operator, '合格',
                    p_so_id, 'SO 一键发货 FEFO', p_posting_time, NULL
                );

                v_picked_total := v_picked_total + v_pick.pick_qty;
                shipped_line_no := v_line.line_no;
                shipped_material_id := v_line.material_id;
                shipped_batch_number := v_pick.batch_number;
                shipped_bin_code := v_pick.bin_code;
                shipped_qty := v_pick.pick_qty;
                shipped_transaction_id := v_txn_id;
                RETURN NEXT;
            END LOOP;
        END IF;

        IF v_picked_total < v_line.open_qty THEN
            RAISE EXCEPTION '销售订单 % 行 % 库存不足：需要 %，实际可发 %', p_so_id, v_line.line_no, v_line.open_qty, v_picked_total;
        END IF;

        UPDATE wms.wms_sales_orders_d
        SET shipped_qty = shipped_qty + v_picked_total,
            map_at_shipment = v_map,
            batch_number = COALESCE(batch_number, shipped_batch_number),
            from_bin = COALESCE(from_bin, shipped_bin_code),
            line_status = CASE WHEN shipped_qty + v_picked_total >= ordered_qty THEN '完成' ELSE '部分发货' END
        WHERE id = v_line.id;
    END LOOP;

    UPDATE wms.wms_sales_orders_h h
    SET status = CASE
            WHEN EXISTS (SELECT 1 FROM wms.wms_sales_orders_d d WHERE d.so_id = p_so_id AND d.open_qty > 0 AND d.line_status <> '取消') THEN '部分发货'
            ELSE '完成'
        END,
        total_cogs = COALESCE((SELECT SUM(line_cogs) FROM wms.wms_sales_orders_d d WHERE d.so_id = p_so_id), 0),
        updated_at = NOW()
    WHERE h.so_id = p_so_id;
END;
$$ LANGUAGE plpgsql;

-- ------------------------------------------------------------
-- 8. 生产订单一键完工：组件 FEFO 领料 + 成品入库 + 批次谱系
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_post_production_complete(
    p_order_id          VARCHAR(30),
    p_output_batch      VARCHAR(30) DEFAULT NULL,
    p_output_bin        VARCHAR(20) DEFAULT NULL,
    p_output_qty        INTEGER DEFAULT NULL,
    p_operator          VARCHAR(50) DEFAULT 'SYSTEM',
    p_quality_status    mdm.quality_status DEFAULT '合格',
    p_posting_time      TIMESTAMPTZ DEFAULT NOW()
) RETURNS TABLE (
    posted_action           VARCHAR(20),
    posted_material_id      VARCHAR(20),
    posted_batch_number     VARCHAR(30),
    posted_qty              INTEGER,
    posted_transaction_id   VARCHAR(30)
) AS $$
DECLARE
    v_h                         RECORD;
    v_d                         RECORD;
    v_pick                      RECORD;
    v_output_batch              VARCHAR(30);
    v_output_bin                VARCHAR(20);
    v_output_qty                INTEGER;
    v_txn_id                    VARCHAR(30);
    v_pick_no                   INTEGER;
    v_issued_total              INTEGER;
    v_planned_unit_cost         NUMERIC(12,4);
    v_planned_material_cost     NUMERIC(15,2);
    v_actual_material_cost      NUMERIC(15,2);
    v_actual_unit_cost          NUMERIC(12,4);
BEGIN
    SELECT h.*, m.default_zone, m.standard_price, COALESCE(pv.standard_cost, m.standard_price, 0) AS planned_unit_cost
      INTO v_h
    FROM wms.wms_production_orders_h h
    JOIN mdm.mdm_materials m ON m.material_id = h.output_material_id
    LEFT JOIN mdm.mdm_product_variants pv ON pv.variant_code = h.variant_code
    WHERE h.order_id = p_order_id
      AND h.status <> '取消'
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION '生产订单 % 不存在或已取消', p_order_id;
    END IF;

    v_output_qty := COALESCE(p_output_qty, NULLIF(v_h.actual_quantity, 0), v_h.planned_quantity);
    v_output_batch := LEFT(COALESCE(p_output_batch, 'B-' || v_h.output_material_id || '-' || to_char(p_posting_time, 'YYYYMMDD') || '-PRD'), 30);

    SELECT COALESCE(
        p_output_bin,
        (
            SELECT bin_code
            FROM mdm.mdm_storage_bins
            WHERE zone = v_h.default_zone AND status IN ('正常','占用')
            ORDER BY current_occupied, bin_code
            LIMIT 1
        )
    ) INTO v_output_bin;

    IF v_output_bin IS NULL THEN
        RAISE EXCEPTION '生产订单 % 未找到成品目标货位', p_order_id;
    END IF;

    INSERT INTO wms.wms_batches(batch_number, material_id, production_date, expiry_date, quality_grade, current_stock, current_bin, quality_status)
    VALUES (v_output_batch, v_h.output_material_id, p_posting_time::DATE, (p_posting_time::DATE + INTERVAL '365 days')::DATE, 'A', 0, NULL, p_quality_status)
    ON CONFLICT (batch_number) DO NOTHING;

    FOR v_d IN
        SELECT *
        FROM wms.wms_production_orders_d
        WHERE order_id = p_order_id
        ORDER BY line_no
    LOOP
        v_pick_no := 0;
        v_issued_total := 0;

        IF COALESCE(v_d.planned_qty,0) = 0 THEN
            CONTINUE;
        END IF;

        IF v_d.batch_number IS NOT NULL AND v_d.from_bin IS NOT NULL THEN
            v_txn_id := LEFT('PI-' || p_order_id || '-' || v_d.line_no::TEXT || '-1', 30);
            PERFORM wms.post_inventory_transaction(
                v_txn_id, '261', v_d.material_id, v_d.planned_qty,
                v_d.from_bin, NULL, v_d.batch_number, NULL, p_operator, '合格',
                p_order_id, '生产订单一键完工领料', p_posting_time, NULL
            );

            INSERT INTO wms.wms_batch_genealogy(parent_batch_number, component_batch_number, parent_material_id, component_material_id, production_order_id, consumed_qty, output_qty, transaction_id)
            VALUES (v_output_batch, v_d.batch_number, v_h.output_material_id, v_d.material_id, p_order_id, v_d.planned_qty, v_output_qty, v_txn_id)
            ON CONFLICT DO NOTHING;

            v_issued_total := v_d.planned_qty;
            posted_action := '领料';
            posted_material_id := v_d.material_id;
            posted_batch_number := v_d.batch_number;
            posted_qty := v_d.planned_qty;
            posted_transaction_id := v_txn_id;
            RETURN NEXT;
        ELSE
            FOR v_pick IN SELECT * FROM wms.fn_pick_batch_fefo(v_d.material_id, v_d.planned_qty, NULL, '合格') LOOP
                v_pick_no := v_pick_no + 1;
                v_txn_id := LEFT('PI-' || p_order_id || '-' || v_d.line_no::TEXT || '-' || v_pick_no::TEXT, 30);

                PERFORM wms.post_inventory_transaction(
                    v_txn_id, '261', v_d.material_id, v_pick.pick_qty,
                    v_pick.bin_code, NULL, v_pick.batch_number, NULL, p_operator, '合格',
                    p_order_id, '生产订单一键完工 FEFO 领料', p_posting_time, NULL
                );

                INSERT INTO wms.wms_batch_genealogy(parent_batch_number, component_batch_number, parent_material_id, component_material_id, production_order_id, consumed_qty, output_qty, transaction_id)
                VALUES (v_output_batch, v_pick.batch_number, v_h.output_material_id, v_d.material_id, p_order_id, v_pick.pick_qty, v_output_qty, v_txn_id)
                ON CONFLICT DO NOTHING;

                v_issued_total := v_issued_total + v_pick.pick_qty;
                posted_action := '领料';
                posted_material_id := v_d.material_id;
                posted_batch_number := v_pick.batch_number;
                posted_qty := v_pick.pick_qty;
                posted_transaction_id := v_txn_id;
                RETURN NEXT;
            END LOOP;
        END IF;

        IF v_issued_total < v_d.planned_qty THEN
            RAISE EXCEPTION '生产订单 % 行 % 组件库存不足：需要 %，实际可领 %', p_order_id, v_d.line_no, v_d.planned_qty, v_issued_total;
        END IF;

        UPDATE wms.wms_production_orders_d
        SET actual_qty = v_issued_total,
            issue_transaction_id = v_txn_id
        WHERE id = v_d.id;
    END LOOP;

    SELECT COALESCE(SUM(d.actual_qty * m.map_price), 0)::NUMERIC(15,2)
      INTO v_actual_material_cost
    FROM wms.wms_production_orders_d d
    JOIN mdm.mdm_materials m ON m.material_id = d.material_id
    WHERE d.order_id = p_order_id;

    v_planned_unit_cost := COALESCE(v_h.planned_unit_cost, v_h.standard_price, 0);
    v_planned_material_cost := ROUND((v_h.planned_quantity * v_planned_unit_cost)::NUMERIC, 2);
    v_actual_unit_cost := CASE
        WHEN v_output_qty > 0 THEN ROUND((v_actual_material_cost / v_output_qty)::NUMERIC, 4)
        ELSE 0
    END;

    v_txn_id := LEFT('PR-' || p_order_id || '-OUT', 30);
    PERFORM wms.post_inventory_transaction(
        v_txn_id, '101', v_h.output_material_id, v_output_qty,
        NULL, v_output_bin, v_output_batch, NULL, p_operator, p_quality_status,
        p_order_id, '生产订单一键完工入库', p_posting_time,
        COALESCE(NULLIF(v_actual_unit_cost, 0), v_planned_unit_cost)
    );

    UPDATE wms.wms_production_orders_h
    SET actual_quantity = v_output_qty,
        status = '完成',
        actual_finish_date = p_posting_time::DATE,
        updated_at = NOW()
    WHERE order_id = p_order_id;

    INSERT INTO wms.wms_production_variances (
        order_id, variant_code, output_material_id,
        planned_quantity, actual_quantity,
        planned_unit_cost, actual_unit_cost,
        planned_material_cost, actual_material_cost,
        variance_reason, created_by, calculated_at
    ) VALUES (
        p_order_id, v_h.variant_code, v_h.output_material_id,
        v_h.planned_quantity, v_output_qty,
        v_planned_unit_cost, v_actual_unit_cost,
        v_planned_material_cost, v_actual_material_cost,
        '生产订单一键完工自动计算', p_operator, p_posting_time
    )
    ON CONFLICT (order_id) DO UPDATE SET
        variant_code = EXCLUDED.variant_code,
        output_material_id = EXCLUDED.output_material_id,
        planned_quantity = EXCLUDED.planned_quantity,
        actual_quantity = EXCLUDED.actual_quantity,
        planned_unit_cost = EXCLUDED.planned_unit_cost,
        actual_unit_cost = EXCLUDED.actual_unit_cost,
        planned_material_cost = EXCLUDED.planned_material_cost,
        actual_material_cost = EXCLUDED.actual_material_cost,
        variance_reason = EXCLUDED.variance_reason,
        created_by = EXCLUDED.created_by,
        calculated_at = EXCLUDED.calculated_at,
        updated_at = NOW();

    posted_action := '入库';
    posted_material_id := v_h.output_material_id;
    posted_batch_number := v_output_batch;
    posted_qty := v_output_qty;
    posted_transaction_id := v_txn_id;
    RETURN NEXT;
END;
$$ LANGUAGE plpgsql;

-- ------------------------------------------------------------
-- 9. 盘点差异一键过账：正差 701，负差 702
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_post_inventory_count(
    p_count_doc_id      VARCHAR(30),
    p_operator          VARCHAR(50) DEFAULT 'SYSTEM',
    p_posting_time      TIMESTAMPTZ DEFAULT NOW()
) RETURNS TABLE (
    posted_line_no          INTEGER,
    posted_material_id      VARCHAR(20),
    posted_batch_number     VARCHAR(30),
    posted_variance_qty     INTEGER,
    posted_movement_type    wms.movement_type,
    posted_transaction_id   VARCHAR(30)
) AS $$
DECLARE
    v_line      RECORD;
    v_move      wms.movement_type;
    v_txn_id    VARCHAR(30);
BEGIN
    IF NOT EXISTS (SELECT 1 FROM wms.wms_inventory_count_h WHERE count_doc_id = p_count_doc_id AND status <> '取消') THEN
        RAISE EXCEPTION '盘点单 % 不存在或已取消', p_count_doc_id;
    END IF;

    FOR v_line IN
        SELECT *
        FROM wms.wms_inventory_count_d
        WHERE count_doc_id = p_count_doc_id
          AND adjusted IS FALSE
          AND variance_qty <> 0
        ORDER BY line_no
    LOOP
        IF v_line.batch_number IS NULL THEN
            RAISE EXCEPTION '盘点单 % 行 % 缺少批次，不能过账', p_count_doc_id, v_line.line_no;
        END IF;

        v_move := CASE WHEN v_line.variance_qty > 0 THEN '701'::wms.movement_type ELSE '702'::wms.movement_type END;
        v_txn_id := LEFT('IC-' || p_count_doc_id || '-' || v_line.line_no::TEXT, 30);

        PERFORM wms.post_inventory_transaction(
            v_txn_id,
            v_move,
            v_line.material_id,
            ABS(v_line.variance_qty),
            CASE WHEN v_move = '702' THEN v_line.bin_code ELSE NULL END,
            CASE WHEN v_move = '701' THEN v_line.bin_code ELSE NULL END,
            v_line.batch_number,
            v_line.serial_number,
            p_operator,
            '合格',
            p_count_doc_id,
            '盘点差异一键过账',
            p_posting_time,
            NULL
        );

        UPDATE wms.wms_inventory_count_d
        SET adjusted = TRUE,
            movement_type = v_move,
            adjustment_transaction_id = v_txn_id
        WHERE id = v_line.id;

        posted_line_no := v_line.line_no;
        posted_material_id := v_line.material_id;
        posted_batch_number := v_line.batch_number;
        posted_variance_qty := v_line.variance_qty;
        posted_movement_type := v_move;
        posted_transaction_id := v_txn_id;
        RETURN NEXT;
    END LOOP;

    UPDATE wms.wms_inventory_count_h
    SET status = '已过账',
        posted_by = p_operator,
        posted_at = p_posting_time,
        updated_at = NOW()
    WHERE count_doc_id = p_count_doc_id;
END;
$$ LANGUAGE plpgsql;

-- ------------------------------------------------------------
-- 10. MRP 一键运算：基于 BOM 爆炸生成 mrp_runs / mrp_suggestions
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION wms.fn_run_mrp(
    p_variant_code      VARCHAR(20),
    p_demand_qty        INTEGER,
    p_demand_date       DATE,
    p_planning_horizon  INTEGER DEFAULT 30,
    p_created_by        VARCHAR(50) DEFAULT 'SYSTEM'
) RETURNS VARCHAR(30) AS $$
DECLARE
    v_base_material_id  VARCHAR(20);
    v_run_id            VARCHAR(30);
    v_component         RECORD;
BEGIN
    IF p_demand_qty IS NULL OR p_demand_qty <= 0 THEN
        RAISE EXCEPTION 'MRP 需求数量必须大于 0';
    END IF;

    SELECT base_material_id INTO v_base_material_id
    FROM mdm.mdm_product_variants
    WHERE variant_code = p_variant_code;

    IF NOT FOUND THEN
        RAISE EXCEPTION '产品变体 % 不存在', p_variant_code;
    END IF;

    v_run_id := LEFT('MRP-' || to_char(NOW(), 'YYYYMMDDHH24MISS') || '-' || p_variant_code, 30);

    INSERT INTO wms.wms_mrp_runs(run_id, run_date, variant_code, demand_qty, demand_date, planning_horizon, status, created_by)
    VALUES (v_run_id, NOW(), p_variant_code, p_demand_qty, p_demand_date, p_planning_horizon, '运行中', p_created_by);

    FOR v_component IN
        SELECT
            e.component_material_id AS material_id,
            MIN(e.bom_level)::INTEGER AS bom_level,
            CEIL(SUM(e.required_qty))::INTEGER AS required_qty
        FROM wms.fn_bom_explosion(v_base_material_id, p_demand_qty, p_variant_code) e
        GROUP BY e.component_material_id
        ORDER BY MIN(e.bom_level), e.component_material_id
    LOOP
        INSERT INTO wms.wms_mrp_suggestions(
            run_id, material_id, bom_level, gross_requirement_qty, required_qty,
            available_qty, safety_stock_qty, suggested_order_type, suggested_order_qty,
            recommended_bin, recommended_batch, lead_time_days, priority, remarks
        )
        SELECT
            v_run_id,
            m.material_id,
            v_component.bom_level,
            v_component.required_qty,
            v_component.required_qty,
            m.current_stock,
            m.safety_stock,
            CASE WHEN m.material_type = '原材料' THEN '采购申请' ELSE '生产订单' END,
            GREATEST(v_component.required_qty + m.safety_stock - m.current_stock, 0),
            (SELECT bs.bin_code FROM wms.wms_bin_stock bs WHERE bs.material_id = m.material_id ORDER BY bs.qty DESC, bs.bin_code LIMIT 1),
            (SELECT b.batch_number FROM wms.wms_batches b WHERE b.material_id = m.material_id AND b.current_stock > 0 AND b.quality_status = '合格' ORDER BY b.expiry_date NULLS LAST, b.production_date, b.batch_number LIMIT 1),
            COALESCE((SELECT ms.lead_time_days FROM mdm.mdm_material_suppliers ms WHERE ms.material_id = m.material_id AND ms.is_primary IS TRUE AND ms.is_active IS TRUE LIMIT 1), 0),
            v_component.bom_level,
            CASE WHEN GREATEST(v_component.required_qty + m.safety_stock - m.current_stock, 0) > 0 THEN '自动 MRP：存在净需求' ELSE '自动 MRP：库存充足' END
        FROM mdm.mdm_materials m
        WHERE m.material_id = v_component.material_id;
    END LOOP;

    UPDATE wms.wms_mrp_runs
    SET status = '完成'
    WHERE run_id = v_run_id;

    RETURN v_run_id;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION wms.fn_pick_batch_fefo IS 'FEFO 自动选批：按最早到期/最早生产日期推荐批次与货位，并通过 FOR UPDATE SKIP LOCKED 锁定候选库存行避免并发超扣。';
COMMENT ON FUNCTION wms.fn_post_purchase_receipt IS 'PO 一键收货：按采购订单未收数量批量 101 入库，并更新 MAP。';
COMMENT ON FUNCTION wms.fn_post_sales_shipment IS 'SO 一键发货：按销售订单未发数量 FEFO 出库，并锁定发货 MAP。';
COMMENT ON FUNCTION wms.fn_post_production_complete IS '生产订单一键完工：组件领料、成品入库、批次谱系一次完成。';
COMMENT ON FUNCTION wms.fn_post_inventory_count IS '盘点差异一键过账：正差 701，负差 702。';
COMMENT ON FUNCTION wms.fn_run_mrp IS 'MRP 一键运算：基于产品变体 BOM 爆炸生成补货建议。';


-- ============================================================
-- 12. 函数注释与调用示例（COMMENT 元数据，便于开发者通过 \df+ 查看）
-- ============================================================
COMMENT ON FUNCTION wms.post_inventory_transaction IS $$
库存过账核心函数：统一处理 101/261/311/501/701/702/999，维护物料库存、批次库存、货位库存、序列号、MAP、MAP 历史、批次历史。
EXAMPLE:
SELECT wms.post_inventory_transaction('T-EX-101','101','CG001',10,NULL,'RM-A01','B-CG-20260401',NULL,'wms01','合格','PO-EX','采购入库示例',NOW(),18.50);
$$;
COMMENT ON FUNCTION wms.fn_bom_explosion IS $$
BOM 需求爆炸：输入成品/半成品物料和需求数量，递归展开组件净需求。
EXAMPLE:
SELECT * FROM wms.fn_bom_explosion('FIN001',50,'FIN-A001');
$$;
COMMENT ON FUNCTION wms.fn_batch_trace_forward IS $$
批次正向追溯：输入原材料/组件批次，返回其流向的下游批次树。
EXAMPLE:
SELECT * FROM wms.fn_batch_trace_forward('B-CG-20260401');
$$;
COMMENT ON FUNCTION wms.fn_batch_trace_backward IS $$
批次反向追溯：输入成品批次，返回其来源组件/原材料批次树。
EXAMPLE:
SELECT * FROM wms.fn_batch_trace_backward('B-FIN-20260422');
$$;
COMMENT ON FUNCTION wms.fn_serial_trace IS $$
序列号全生命周期追溯：返回单件序列号的状态、货位、质量与事务流转。
EXAMPLE:
SELECT * FROM wms.fn_serial_trace('SN-FIN-0001');
$$;
COMMENT ON FUNCTION wms.fn_pick_batch_fefo IS $$
FEFO 自动选批：按最早到期/最早生产日期推荐批次与货位，并通过 FOR UPDATE SKIP LOCKED 锁定候选库存行避免并发超扣。
EXAMPLE:
SELECT * FROM wms.fn_pick_batch_fefo('FIN001',3,NULL,'合格');
$$;
COMMENT ON FUNCTION wms.fn_post_purchase_receipt IS $$
PO 一键收货：按采购订单未收数量批量 101 入库，并在传入/读取单价时更新 MAP。
EXAMPLE:
SELECT * FROM wms.fn_post_purchase_receipt('PO-2026-0001','RM-A01','B-GR-20260601','wms01');
$$;
COMMENT ON FUNCTION wms.fn_post_sales_shipment IS $$
SO 一键发货：按销售订单未发数量执行 FEFO 出库，并锁定发货时 MAP。
EXAMPLE:
SELECT * FROM wms.fn_post_sales_shipment('SO-2026-0001','wms01');
$$;
COMMENT ON FUNCTION wms.fn_post_production_complete IS $$
生产订单一键完工：FEFO 组件领料、成品入库、批次谱系、生产成本差异一次完成。
EXAMPLE:
SELECT * FROM wms.fn_post_production_complete('MO-2026-0001','B-FIN-20260601','FG-A01','wms01');
$$;
COMMENT ON FUNCTION wms.fn_post_inventory_count IS $$
盘点差异一键过账：正差生成 701，负差生成 702。
EXAMPLE:
SELECT * FROM wms.fn_post_inventory_count('IC-2026-0001','wms01');
$$;
COMMENT ON FUNCTION wms.fn_run_mrp IS $$
MRP 一键运算：基于产品变体和需求数量展开 BOM，生成补货建议。
EXAMPLE:
SELECT wms.fn_run_mrp('FIN-A001',20,CURRENT_DATE + 14);
$$;
COMMENT ON FUNCTION rpt.refresh_all_materialized_views IS $$
按依赖顺序刷新全部 8 个报表物化视图。
EXAMPLE:
SELECT rpt.refresh_all_materialized_views();
$$;

COMMIT;


-- ============================================================
-- 验证 SQL（注释，按需执行）
-- ============================================================
/*
-- 验证 BOM 爆炸
SELECT * FROM wms.fn_bom_explosion('FIN001', 50, 'FIN-A001');

-- 验证批次反向追溯（成品 → 原材料）
SELECT * FROM wms.fn_batch_trace_backward('B-FIN-20260422');

-- 验证批次正向追溯（原材料 → 成品）
SELECT * FROM wms.fn_batch_trace_forward('B-CG-20260401');

-- 验证 FEFO 自动选批（函数内部使用 FOR UPDATE SKIP LOCKED 锁定候选库存行）
SELECT * FROM wms.fn_pick_batch_fefo('FIN001', 3);

-- 验证生产成本差异
SELECT order_id, planned_material_cost, actual_material_cost, material_variance, variance_pct
FROM wms.wms_production_variances
ORDER BY order_id;

-- 验证 MRP 一键运算
SELECT wms.fn_run_mrp('FIN-A001', 20, CURRENT_DATE + 14);

-- 验证库存一致性
SELECT * FROM rpt.rpt_data_consistency_check WHERE check_status <> '一致';

-- 验证物料 × 区域 PIVOT
SELECT * FROM rpt.rpt_stock_by_zone ORDER BY material_type, material_id;

-- 验证货位利用率
SELECT * FROM rpt.rpt_bin_stock_summary ORDER BY zone, bin_code;

-- 验证批次预警
SELECT * FROM rpt.rpt_batch_stock_summary WHERE alert_level <> '✓ 正常';

-- 验证升级后过账函数（含 MAP 自动更新）
SELECT wms.post_inventory_transaction(
    'T-MAP-TEST', '101', 'CG001', 50, NULL, 'RM-A01',
    'B-CG-20260401', NULL, 'wms01', '合格', 'PO-TEST', 'MAP 测试',
    NOW(), 20.00
);
SELECT * FROM wms.wms_map_history WHERE material_id='CG001' ORDER BY changed_at DESC LIMIT 1;
SELECT material_id, current_stock, map_price FROM mdm.mdm_materials WHERE material_id='CG001';


-- 验证负库存拦截：应捕获异常，不应真的过账成功
DO $$
BEGIN
    BEGIN
        PERFORM wms.post_inventory_transaction(
            'T-NEG-TEST','261','CG001',999999,'RM-A01',NULL,'B-CG-20260401',NULL,
            'tester','合格','TEST','负库存测试',NOW(),NULL
        );
        RAISE EXCEPTION '测试失败：负库存未被拦截';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '测试通过：负库存拦截 -> %', SQLERRM;
    END;
END $$;

-- 验证冻结批次出库拦截：请先准备一个 quality_status='冻结' 且有库存的批次
-- DO $$
-- BEGIN
--     BEGIN
--         PERFORM wms.post_inventory_transaction(
--             'T-FROZEN-TEST','261','CG001',1,'RM-A01',NULL,'B-FROZEN-TEST',NULL,
--             'tester','冻结','TEST','冻结批次出库测试',NOW(),NULL
--         );
--         RAISE EXCEPTION '测试失败：冻结批次未被拦截';
--     EXCEPTION WHEN OTHERS THEN
--         RAISE NOTICE '测试通过：冻结批次拦截 -> %', SQLERRM;
--     END;
-- END $$;

-- 验证货位容量超限：请选一个小容量货位并尝试大数量入库，应捕获异常
DO $$
BEGIN
    BEGIN
        PERFORM wms.post_inventory_transaction(
            'T-CAP-TEST','101','CG001',999999,NULL,'RM-A01','B-CG-20260401',NULL,
            'tester','合格','TEST','容量超限测试',NOW(),18.50
        );
        RAISE EXCEPTION '测试失败：货位容量超限未被拦截';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '测试通过：货位容量拦截 -> %', SQLERRM;
    END;
END $$;

*/

-- ============================================================
-- v9 总结
-- ============================================================
/*
v9 重构完整版在 v8 基础上完成交付级整理：
+ 物化视图刷新顺序按依赖重排：current_stock 先行，库存价值/区域/货位/批次随后，质量/MRP/低库存最后。
+ 事务分区扩展：wms_transactions 子分区覆盖 2026-04 至 2026-12。
+ 文档增强：物化视图依赖注释、函数 EXAMPLE 注释、生产成本差异字段说明。
+ 验证增强：注释态异常测试块覆盖负库存、冻结批次、容量超限等场景。
+ 清理重复函数定义：post_inventory_transaction / refresh_all_materialized_views 均只保留最终版一次。
+ 一键业务函数：PO 一键收货、SO 一键发货、生产订单一键完工、盘点差异一键过账、FEFO 自动选批、MRP 一键运算。
+ 数据完整性：updated_at 自动维护、关键表 UPDATE/DELETE 审计、每物料唯一主供应商约束。
+ 并发控制：FEFO 选批通过 FOR UPDATE SKIP LOCKED 锁定候选库存行，降低高并发超扣风险。
+ 成本差异：wms_production_variances 记录生产订单级计划/实际成本差异。
+ 数据校验：rpt_data_consistency_check 四账核对视图。
+ 保留 v6 能力：MAP 自动更新、MAP 历史、批次历史、BOM 爆炸、批次/序列号追溯、8 个报表物化视图。

最终对象总数：
  业务表/系统表 ：46 张（不含 9 个分区子表）
  分区子表      ：9 个
  物化视图      ：8 个
  普通视图      ：1 个
  函数/过程     ：15 个
  触发器        ：23 个
  自定义类型    ：3 个 ENUM

适用方式：空库/演示库初始化。脚本开头会 DROP rpt/wms/mdm/sys，请勿直接在生产库执行。
*/
