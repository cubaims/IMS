-- Quality module access was added after some demo databases had already been
-- initialized. Keep this migration idempotent because older environments may
-- already contain part of this seed data.

INSERT INTO sys.sys_user_permissions (role_id, permission_code, permission_name, granted_by)
SELECT role_id, permission_code, permission_name, 'SYSTEM'
FROM (
    VALUES
        ('ADMIN', 'quality:read', '质量查询'),
        ('ADMIN', 'quality:write', '质量维护'),
        ('ADMIN', 'quality:decision', '质量判定'),
        ('QM_USER', 'quality:read', '质量查询'),
        ('QM_USER', 'quality:write', '质量维护'),
        ('QM_USER', 'quality:decision', '质量判定')
) AS seed(role_id, permission_code, permission_name)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys.sys_user_permissions existing
    WHERE existing.role_id = seed.role_id
      AND existing.permission_code = seed.permission_code
);

-- The original demo seed used a placeholder that cannot pass Argon2
-- verification. Only replace non-Argon2 demo placeholders; do not overwrite
-- accounts whose passwords were already initialized by an operator.
UPDATE sys.sys_users
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$NxGPwpyeyIOJh7aEyiSmvA$l6/7S0/yUg14XQAQ+xOUW4xu+kIYWTveVxLCi+aXk48'
WHERE username IN (
    'admin',
    'wms01',
    'warehouse01',
    'purchaser01',
    'sales01',
    'auditor01',
    'qm01',
    'planner01'
)
  AND NOT starts_with(password_hash, chr(36) || 'argon2');
