-- 邀请码可指定注册角色：仅 employee / canteen，不经邀请码发 admin / super_admin。
-- 旧行默认 employee + '{}'（兼容已有邀请码）。
ALTER TABLE invite_code
    ADD COLUMN role TEXT NOT NULL DEFAULT 'employee',
    ADD COLUMN canteen_ids UUID[] NOT NULL DEFAULT '{}';

ALTER TABLE invite_code
    ADD CONSTRAINT invite_role_chk CHECK (role IN ('employee', 'canteen')),
    ADD CONSTRAINT invite_role_scope_chk CHECK (
        (role = 'employee' AND cardinality(canteen_ids) = 0)
        OR (role = 'canteen' AND cardinality(canteen_ids) >= 1)
    );
