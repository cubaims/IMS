-- Grant unified traceability read access to roles that already need batch,
-- inventory, quality, or audit trace views.
INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, 'traceability:read', '追溯查询', 'SYSTEM'
FROM (VALUES
    ('WMS_USER'),
    ('WAREHOUSE_OPERATOR'),
    ('AUDITOR'),
    ('QM_USER'),
    ('PLANNER')
) AS roles(role_id)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.user_id IS NULL
      AND existing.role_id = roles.role_id
      AND existing.permission_code = 'traceability:read'
);
