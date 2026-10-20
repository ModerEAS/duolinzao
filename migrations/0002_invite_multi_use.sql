-- 邀请码可多次使用：发放时定上限，用完作废，可改上限、可提前作废。
-- 旧行默认 max_uses=1（兼容旧行为）；已用过的旧码回填 used_count=1。
ALTER TABLE invite_code
    ADD COLUMN max_uses INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN used_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN revoked_at TIMESTAMPTZ;

UPDATE invite_code SET used_count = 1 WHERE used_at IS NOT NULL;

ALTER TABLE invite_code
    ADD CONSTRAINT invite_uses_chk CHECK (max_uses >= 1 AND used_count >= 0 AND used_count <= max_uses);
