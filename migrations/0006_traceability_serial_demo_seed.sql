-- Seed a real serial-number trace path for the traceability UI.
--
-- The batch trace demo already uses B-FIN-20260422. This migration adds one
-- serialized finished good against that same batch so `/api/traceability/serials/SN-FIN-0001`
-- returns bound material, bound batch, inventory movements, serial history, and
-- batch context.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM wms.wms_serial_numbers
        WHERE serial_number = 'SN-FIN-0001'
    )
    AND EXISTS (
        SELECT 1
        FROM wms.wms_batches
        WHERE batch_number = 'B-FIN-20260422'
          AND material_id = 'FIN001'
    )
    AND NOT EXISTS (
        SELECT 1
        FROM wms.wms_transactions
        WHERE transaction_id IN ('TSN-FIN-0001-101', 'TSN-FIN-0001-311', 'TSN-FIN-0001-261')
    ) THEN
        -- wms.wms_transactions.serial_number has an FK to wms.wms_serial_numbers.
        -- The posting function updates serial state after writing the transaction,
        -- so seed the serial shell first and let the function write movements/history.
        INSERT INTO wms.wms_serial_numbers (
            serial_number,
            material_id,
            batch_number,
            current_status,
            current_bin,
            quality_status,
            last_movement_at
        ) VALUES (
            'SN-FIN-0001',
            'FIN001',
            'B-FIN-20260422',
            '在库',
            NULL,
            '合格',
            NULL
        );

        PERFORM wms.post_inventory_transaction(
            'TSN-FIN-0001-101',
            '101',
            'FIN001',
            1,
            NULL,
            'FG-D01',
            'B-FIN-20260422',
            'SN-FIN-0001',
            'wms01',
            '合格',
            'PRD-FIN-0001',
            'SN-FIN-0001 生产入库',
            '2026-04-22 17:05:00+08'
        );

        PERFORM wms.post_inventory_transaction(
            'TSN-FIN-0001-311',
            '311',
            'FIN001',
            1,
            'FG-D01',
            'FG-D02',
            'B-FIN-20260422',
            'SN-FIN-0001',
            'wms01',
            '合格',
            'SO-2026-0001',
            'SN-FIN-0001 转储到发货区',
            '2026-05-01 09:05:00+08'
        );

        PERFORM wms.post_inventory_transaction(
            'TSN-FIN-0001-261',
            '261',
            'FIN001',
            1,
            'FG-D02',
            NULL,
            'B-FIN-20260422',
            'SN-FIN-0001',
            'wms01',
            '合格',
            'SO-2026-0001',
            'SN-FIN-0001 销售出库',
            '2026-05-02 10:05:00+08'
        );
    END IF;

    IF EXISTS (
        SELECT 1
        FROM wms.wms_transactions
        WHERE transaction_id = 'TSN-FIN-0001-261'
          AND serial_number = 'SN-FIN-0001'
    )
    AND EXISTS (
        SELECT 1
        FROM wms.wms_serial_numbers
        WHERE serial_number = 'SN-FIN-0001'
          AND current_status <> '已销售'
    ) THEN
        INSERT INTO wms.wms_serial_history (
            serial_number,
            old_status,
            new_status,
            old_bin,
            new_bin,
            old_quality_status,
            new_quality_status,
            transaction_id,
            changed_by,
            changed_at
        )
        SELECT
            serial_number,
            current_status,
            '已销售',
            current_bin,
            NULL,
            quality_status,
            quality_status,
            'TSN-FIN-0001-261',
            'wms01',
            '2026-05-02 10:05:01+08'
        FROM wms.wms_serial_numbers
        WHERE serial_number = 'SN-FIN-0001'
          AND NOT EXISTS (
              SELECT 1
              FROM wms.wms_serial_history
              WHERE serial_number = 'SN-FIN-0001'
                AND transaction_id = 'TSN-FIN-0001-261'
                AND old_status = '生产中'
                AND new_status = '已销售'
          );

        UPDATE wms.wms_serial_numbers
        SET current_status = '已销售',
            current_bin = NULL,
            updated_at = NOW()
        WHERE serial_number = 'SN-FIN-0001';
    END IF;

    UPDATE wms.wms_serial_history h
    SET changed_at = t.transaction_date
    FROM wms.wms_transactions t
    WHERE h.serial_number = 'SN-FIN-0001'
      AND h.transaction_id = t.transaction_id
      AND h.transaction_id IN ('TSN-FIN-0001-101', 'TSN-FIN-0001-311', 'TSN-FIN-0001-261')
      AND h.new_status <> '已销售';

    UPDATE wms.wms_serial_history
    SET changed_at = '2026-05-02 10:05:01+08'
    WHERE serial_number = 'SN-FIN-0001'
      AND transaction_id = 'TSN-FIN-0001-261'
      AND old_status = '生产中'
      AND new_status = '已销售';
END $$;
