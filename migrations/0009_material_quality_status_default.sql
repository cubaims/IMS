-- Phase 3 material master-data quality default rule.
--
-- Material quality_status is the default quality status for newly created
-- material-related stock/batch flows. Runtime stock quality remains owned by
-- WMS batch/bin-stock tables.

ALTER TABLE mdm.mdm_materials
    ADD COLUMN IF NOT EXISTS quality_status mdm.quality_status;

UPDATE mdm.mdm_materials
SET quality_status = '合格'::mdm.quality_status
WHERE quality_status IS NULL;

ALTER TABLE mdm.mdm_materials
    ALTER COLUMN quality_status SET DEFAULT '合格'::mdm.quality_status,
    ALTER COLUMN quality_status SET NOT NULL;
