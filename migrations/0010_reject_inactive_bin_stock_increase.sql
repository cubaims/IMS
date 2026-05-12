-- Phase 3 storage-bin rule hardening:
-- a disabled/maintenance bin must not receive new stock, even if a caller
-- bypasses module-level checks and writes through the generic posting function.

CREATE OR REPLACE FUNCTION wms.fn_reject_inactive_bin_stock_increase()
RETURNS TRIGGER AS $$
DECLARE
    v_status VARCHAR(20);
    v_should_check BOOLEAN := FALSE;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_should_check := COALESCE(NEW.qty, 0) > 0;
    ELSE
        v_should_check :=
            NEW.bin_code IS DISTINCT FROM OLD.bin_code
            OR COALESCE(NEW.qty, 0) > COALESCE(OLD.qty, 0);
    END IF;

    IF NOT v_should_check THEN
        RETURN NEW;
    END IF;

    SELECT status
      INTO v_status
    FROM mdm.mdm_storage_bins
    WHERE bin_code = NEW.bin_code;

    IF NOT FOUND THEN
        RAISE EXCEPTION '目标货位 % 不存在', NEW.bin_code;
    END IF;

    IF v_status NOT IN ('正常', '占用') THEN
        RAISE EXCEPTION '目标货位 % 不可用，当前状态 %', NEW.bin_code, v_status;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_reject_inactive_bin_stock_increase ON wms.wms_bin_stock;

CREATE TRIGGER trg_reject_inactive_bin_stock_increase
BEFORE INSERT OR UPDATE OF bin_code, qty ON wms.wms_bin_stock
FOR EACH ROW
EXECUTE FUNCTION wms.fn_reject_inactive_bin_stock_increase();
