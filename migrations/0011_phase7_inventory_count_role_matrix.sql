-- Align Phase 7 inventory-count role grants with the route-level permission
-- boundaries enforced by cuba-api.

INSERT INTO sys.sys_roles (role_id, role_name, description)
SELECT role_id, role_name, description
FROM (VALUES
    ('WAREHOUSE_MANAGER', '仓库主管', '审核、过账并关闭盘点单')
) AS roles(role_id, role_name, description)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_roles existing
    WHERE existing.role_id = roles.role_id
);

INSERT INTO sys.sys_users (username, password_hash, full_name, email, role_id)
SELECT username, password_hash, full_name, email, role_id
FROM (VALUES
    (
        'warehouse_manager01',
        'demo-not-for-production',
        '仓库主管',
        'warehouse_manager01@example.com',
        'WAREHOUSE_MANAGER'
    )
) AS users(username, password_hash, full_name, email, role_id)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_users existing
    WHERE existing.username = users.username
);

INSERT INTO sys.sys_user_roles (user_id, role_id, assigned_by)
SELECT u.user_id, u.role_id, 'SYSTEM'
FROM sys.sys_users u
WHERE u.role_id = 'WAREHOUSE_MANAGER'
  AND NOT EXISTS (
      SELECT 1
      FROM sys.sys_user_roles existing
      WHERE existing.user_id = u.user_id
        AND existing.role_id = u.role_id
  );

DELETE FROM sys.sys_user_permissions
WHERE user_id IS NULL
  AND role_id = 'WAREHOUSE_OPERATOR'
  AND permission_code IN (
      'inventory-count:approve',
      'inventory-count:post',
      'inventory-count:close'
  );

INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, permission_code, permission_name, 'SYSTEM'
FROM (VALUES
    ('WAREHOUSE_OPERATOR', 'inventory-count:read', '盘点查询'),
    ('WAREHOUSE_OPERATOR', 'inventory-count:write', '盘点维护'),
    ('WAREHOUSE_OPERATOR', 'inventory-count:submit', '盘点提交'),
    ('WAREHOUSE_MANAGER', 'inventory:read', '库存查询'),
    ('WAREHOUSE_MANAGER', 'inventory:post', '库存过账'),
    ('WAREHOUSE_MANAGER', 'inventory-count:read', '盘点查询'),
    ('WAREHOUSE_MANAGER', 'inventory-count:approve', '盘点审核'),
    ('WAREHOUSE_MANAGER', 'inventory-count:post', '盘点过账'),
    ('WAREHOUSE_MANAGER', 'inventory-count:close', '盘点关闭'),
    ('AUDITOR', 'inventory-count:read', '盘点查询')
) AS grants(role_id, permission_code, permission_name)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.user_id IS NULL
      AND existing.role_id = grants.role_id
      AND existing.permission_code = grants.permission_code
);
