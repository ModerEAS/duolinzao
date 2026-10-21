-- 应用签名 keystore（单例行）：Android 正式包的签名材料只活在这里，由超管上传/替换。
-- keystore 本体与两个密码都经 local_key AES-GCM 加密后存放（见 crypto::encrypt，
-- 与 llm_channel.key_ciphertext 同模式）；key alias 不是秘密，明文存。
-- 数据库备份即包含签名密钥：备份必须离线保护，丢了/换了 key，已装设备只能卸载重装。
CREATE TABLE app_signing (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    keystore_ciphertext BYTEA NOT NULL,
    keystore_password_ciphertext BYTEA NOT NULL,
    key_alias TEXT NOT NULL,
    key_password_ciphertext BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
