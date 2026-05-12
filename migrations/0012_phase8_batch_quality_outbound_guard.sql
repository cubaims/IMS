-- Phase 8 quality hardening:
-- outbound-like inventory movements must use the real batch quality status,
-- and only qualified batches may leave or move stock.

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
    p_unit_price        NUMERIC(12,2) DEFAULT NULL
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
    v_old_map               NUMERIC(12,2);
    v_new_map               NUMERIC(12,2);
    v_old_total_value       NUMERIC(15,2);
    v_received_amount       NUMERIC(15,2);
    v_formula               TEXT;
BEGIN
    IF p_quantity IS NULL OR p_quantity <= 0 THEN
        RAISE EXCEPTION '过账数量必须大于 0';
    END IF;

    SELECT * INTO v_cfg
    FROM wms.wms_movement_type_config
    WHERE movement_type = p_movement_type;

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
        WHERE batch_number = p_batch_number
          AND material_id = p_material_id
        FOR UPDATE;

        IF NOT FOUND THEN
            RAISE EXCEPTION '批次 % 不存在或不属于物料 %', p_batch_number, p_material_id;
        END IF;

        IF p_movement_type IN ('261', '311', '702', '999') THEN
            IF v_batch_status = '待检' THEN
                RAISE EXCEPTION '批次 % 当前质量状态为 待检，禁止出库/转移/报废过账'
                    USING ERRCODE = 'P0001', HINT = 'BATCH_PENDING_INSPECTION';
            ELSIF v_batch_status = '冻结' THEN
                RAISE EXCEPTION '批次 % 当前质量状态为 冻结，禁止出库/转移/报废过账'
                    USING ERRCODE = 'P0001', HINT = 'BATCH_FROZEN';
            ELSIF v_batch_status = '报废' THEN
                RAISE EXCEPTION '批次 % 当前质量状态为 报废，禁止出库/转移/报废过账'
                    USING ERRCODE = 'P0001', HINT = 'BATCH_SCRAPPED';
            ELSIF v_batch_status <> '合格' THEN
                RAISE EXCEPTION '批次 % 当前质量状态为 %，禁止出库/转移/报废过账',
                    p_batch_number, v_batch_status;
            END IF;

            v_effective_quality := v_batch_status;
        ELSE
            v_effective_quality := COALESCE(p_quality_status, v_batch_status);
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

    CASE p_movement_type
        WHEN '101' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '501' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '701' THEN v_material_delta:=p_quantity;  v_batch_delta:=p_quantity;  v_txn_quantity:=p_quantity;
        WHEN '261' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '702' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '999' THEN v_material_delta:=-p_quantity; v_batch_delta:=-p_quantity; v_txn_quantity:=-p_quantity;
        WHEN '311' THEN v_material_delta:=0;           v_batch_delta:=0;           v_txn_quantity:=p_quantity;
    END CASE;

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
        WHERE material_id = p_material_id
          AND bin_code = p_from_bin
          AND batch_number = p_batch_number
        FOR UPDATE;

        IF NOT FOUND OR v_from_stock < p_quantity THEN
            RAISE EXCEPTION '货位 % 物料 % 批次 % 库存不足',
                p_from_bin, p_material_id, p_batch_number;
        END IF;
    END IF;

    IF p_to_bin IS NOT NULL THEN
        SELECT capacity, current_occupied
          INTO v_capacity, v_occupied
        FROM mdm.mdm_storage_bins
        WHERE bin_code = p_to_bin
        FOR UPDATE;

        IF NOT FOUND THEN
            RAISE EXCEPTION '目标货位 % 不存在', p_to_bin;
        END IF;

        IF v_occupied + p_quantity > v_capacity THEN
            RAISE EXCEPTION '目标货位 % 容量不足', p_to_bin;
        END IF;
    END IF;

    INSERT INTO wms.wms_transactions (
        transaction_id, transaction_date, movement_type, material_id, quantity,
        from_bin, to_bin, batch_number, serial_number, reference_doc,
        operator, quality_status, notes
    ) VALUES (
        p_transaction_id, p_transaction_date, p_movement_type, p_material_id, v_txn_quantity,
        p_from_bin, p_to_bin, p_batch_number, p_serial_number, p_reference_doc,
        p_operator, v_effective_quality, p_notes
    );

    IF p_from_bin IS NOT NULL THEN
        UPDATE wms.wms_bin_stock
        SET qty = qty - p_quantity,
            updated_at = NOW()
        WHERE material_id = p_material_id
          AND bin_code = p_from_bin
          AND batch_number = p_batch_number;

        UPDATE mdm.mdm_storage_bins
        SET current_occupied = current_occupied - p_quantity,
            updated_at = NOW()
        WHERE bin_code = p_from_bin;
    END IF;

    IF p_to_bin IS NOT NULL THEN
        INSERT INTO wms.wms_bin_stock (material_id, bin_code, batch_number, quality_status, qty)
        VALUES (p_material_id, p_to_bin, p_batch_number, v_effective_quality, p_quantity)
        ON CONFLICT (material_id, bin_code, batch_number)
        DO UPDATE SET
            qty = wms.wms_bin_stock.qty + EXCLUDED.qty,
            quality_status = EXCLUDED.quality_status,
            updated_at = NOW();

        UPDATE mdm.mdm_storage_bins
        SET current_occupied = current_occupied + p_quantity,
            updated_at = NOW()
        WHERE bin_code = p_to_bin;
    END IF;

    DELETE FROM wms.wms_bin_stock WHERE qty = 0;

    IF p_movement_type IN ('101','501','701') AND p_unit_price IS NOT NULL THEN
        v_old_total_value := COALESCE(v_material_stock, 0) * COALESCE(v_old_map, 0);
        v_received_amount := p_quantity * p_unit_price;

        IF (v_material_stock + p_quantity) > 0 THEN
            v_new_map := ROUND(
                (v_old_total_value + v_received_amount)::NUMERIC
                / (v_material_stock + p_quantity),
                4
            );
        ELSE
            v_new_map := v_old_map;
        END IF;

        v_formula := format(
            '(%s × %s + %s × %s) / (%s + %s) = %s',
            v_material_stock, COALESCE(v_old_map, 0),
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

    UPDATE mdm.mdm_materials
    SET current_stock = current_stock + v_material_delta,
        map_price = CASE
            WHEN p_movement_type IN ('101','501','701') AND p_unit_price IS NOT NULL
            THEN v_new_map
            ELSE map_price
        END,
        updated_at = NOW()
    WHERE material_id = p_material_id;

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

    IF p_serial_number IS NOT NULL THEN
        SELECT current_status, current_bin, quality_status
          INTO v_old_serial_status, v_old_serial_bin, v_old_serial_quality
        FROM wms.wms_serial_numbers
        WHERE serial_number = p_serial_number
        FOR UPDATE;

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
                last_movement_at = NOW(),
                updated_at = NOW()
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
    '库存过账核心函数 v6 + Phase 8 quality guard - outbound movements require qualified batch status';
