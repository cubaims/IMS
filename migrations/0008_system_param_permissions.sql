-- System parameters use a separate permission boundary from audit logs.
-- This keeps audit history access from implicitly granting configuration
-- maintenance access.

INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, permission_code, permission_name, 'SYSTEM'
FROM (
    VALUES
        ('ADMIN', 'system-param:read', '系统参数查询'),
        ('ADMIN', 'system-param:write', '系统参数维护')
) AS seed(role_id, permission_code, permission_name)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.role_id = seed.role_id
      AND existing.permission_code = seed.permission_code
);
