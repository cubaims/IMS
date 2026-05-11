-- Grant real audit-log read access to system administrators and auditors.
INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, 'audit:read', '审计日志查询', 'SYSTEM'
FROM (VALUES ('ADMIN'), ('AUDITOR')) AS roles(role_id)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.user_id IS NULL
      AND existing.role_id = roles.role_id
      AND existing.permission_code = 'audit:read'
);
