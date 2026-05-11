-- Grant Phase 4 inventory-core permissions that are enforced by cuba-api routes.
INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, permission_code, permission_name, 'SYSTEM'
FROM (VALUES
    ('WMS_USER', 'inventory:history', '库存流水查询'),
    ('WMS_USER', 'inventory:transfer', '库位转储'),
    ('WMS_USER', 'batch:read', '批次查询'),
    ('WMS_USER', 'batch:history', '批次历史查询'),
    ('WMS_USER', 'cost:map-read', '移动平均价历史查询'),
    ('WAREHOUSE_OPERATOR', 'inventory:history', '库存流水查询'),
    ('WAREHOUSE_OPERATOR', 'inventory:transfer', '库位转储'),
    ('WAREHOUSE_OPERATOR', 'batch:read', '批次查询'),
    ('WAREHOUSE_OPERATOR', 'batch:history', '批次历史查询'),
    ('PURCHASER', 'batch:read', '批次查询'),
    ('SALES_OPERATOR', 'batch:read', '批次查询'),
    ('AUDITOR', 'inventory:history', '库存流水查询'),
    ('AUDITOR', 'batch:read', '批次查询'),
    ('AUDITOR', 'batch:history', '批次历史查询'),
    ('AUDITOR', 'cost:map-read', '移动平均价历史查询')
) AS grants(role_id, permission_code, permission_name)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.user_id IS NULL
      AND existing.role_id = grants.role_id
      AND existing.permission_code = grants.permission_code
);
