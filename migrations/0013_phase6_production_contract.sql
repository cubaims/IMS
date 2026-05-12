-- Phase 6 production contract hardening.
-- Keep this migration idempotent because some environments may already carry
-- manually patched production columns or role grants.

ALTER TABLE wms.wms_production_orders_h
    ADD COLUMN IF NOT EXISTS remark TEXT;

ALTER TABLE wms.wms_production_orders_h
    DROP CONSTRAINT IF EXISTS wms_production_orders_h_status_check;

ALTER TABLE wms.wms_production_orders_h
    ADD CONSTRAINT wms_production_orders_h_status_check
    CHECK (status IN ('计划中', '已下达', '生产中', '完成', '关闭', '取消'));

INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, permission_code, permission_name, 'SYSTEM'
FROM (
    VALUES
        ('ADMIN', 'production:read', '生产查询'),
        ('ADMIN', 'production:write', '生产维护'),
        ('ADMIN', 'production:release', '生产订单下达'),
        ('ADMIN', 'production:complete', '生产完工'),
        ('ADMIN', 'production:variance-read', '生产差异查询'),
        ('ADMIN', 'bom:explode', 'BOM 展开'),
        ('ADMIN', 'batch:trace', '批次追溯'),
        ('PLANNER', 'production:variance-read', '生产差异查询'),
        ('AUDITOR', 'production:read', '生产查询'),
        ('AUDITOR', 'production:variance-read', '生产差异查询'),
        ('AUDITOR', 'batch:trace', '批次追溯')
) AS seed(role_id, permission_code, permission_name)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.role_id = seed.role_id
      AND existing.permission_code = seed.permission_code
)
AND EXISTS (
    SELECT 1
    FROM sys.sys_roles roles
    WHERE roles.role_id = seed.role_id
);
