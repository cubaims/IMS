-- Phase 5 Verification SQL Queries

-- 最近 20 条库存事务
SELECT
    transaction_id,
    movement_type,
    material_id,
    quantity,
    from_bin,
    to_bin,
    batch_number,
    reference_doc,
    operator,
    transaction_date
FROM wms.wms_transactions
ORDER BY transaction_date DESC
LIMIT 20;

-- 检查 CG001 当前库存
SELECT
    material_id,
    material_name,
    current_stock,
    map_price
FROM mdm.mdm_materials
WHERE material_id = 'CG001';

-- 检查 CG001 货位库存
SELECT
    material_id,
    bin_code,
    batch_number,
    qty,
    quality_status
FROM wms.wms_bin_stock
WHERE material_id = 'CG001'
ORDER BY bin_code, batch_number;

-- 检查 CG001 批次库存
SELECT
    batch_number,
    material_id,
    current_stock,
    current_bin,
    quality_status,
    production_date,
    expiry_date
FROM wms.wms_batches
WHERE material_id = 'CG001'
ORDER BY expiry_date NULLS LAST, production_date;

-- 检查 MAP 历史
SELECT
    material_id,
    old_map_price,
    new_map_price,
    incoming_qty,
    incoming_unit_price,
    transaction_id,
    changed_at
FROM wms.wms_map_history
WHERE material_id = 'CG001'
ORDER BY changed_at DESC
LIMIT 20;

-- 检查 PO 状态
SELECT
    h.po_id,
    h.status,
    d.line_no,
    d.material_id,
    d.ordered_qty,
    d.received_qty,
    d.open_qty,
    d.line_status
FROM wms.wms_purchase_orders_h h
JOIN wms.wms_purchase_orders_d d ON d.po_id = h.po_id
ORDER BY h.created_at DESC
LIMIT 20;

-- 检查 SO 状态
SELECT
    h.so_id,
    h.status,
    d.line_no,
    d.material_id,
    d.ordered_qty,
    d.shipped_qty,
    d.open_qty,
    d.line_status
FROM wms.wms_sales_orders_h h
JOIN wms.wms_sales_orders_d d ON d.so_id = h.so_id
ORDER BY h.created_at DESC
LIMIT 20;

-- 刷新报表后检查库存一致性
SELECT rpt.refresh_all_materialized_views();

SELECT * FROM rpt.rpt_data_consistency_check
WHERE check_status <> '一致';
