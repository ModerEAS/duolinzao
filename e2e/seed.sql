-- e2e 最小数据：一个食堂 + 一张未使用的邀请码。邀请码库内存哈希（sha256 hex）。
-- 由 scripts/e2e.sh 在 app 启动（迁移 + bootstrap 超管）之后执行。
INSERT INTO canteen (name)
SELECT :'canteen'
WHERE NOT EXISTS (SELECT 1 FROM canteen WHERE name = :'canteen');

-- 第二食堂：给 admin 范围内外对照、改单跨食堂用（幂等）。
INSERT INTO canteen (name)
SELECT :'canteen2'
WHERE NOT EXISTS (SELECT 1 FROM canteen WHERE name = :'canteen2');

INSERT INTO invite_code (code_hash, created_by)
SELECT :'hash', id FROM account WHERE username = :'super_user'
ON CONFLICT (code_hash) DO NOTHING;

-- 码固定，本地可反复跑：每次重置为未使用。
UPDATE invite_code SET used_by = NULL, used_at = NULL WHERE code_hash = :'hash';
