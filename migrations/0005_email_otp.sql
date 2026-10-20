-- 邮箱 OTP（取代 magic link）+ 认证器 TOTP 防重放。
CREATE TABLE IF NOT EXISTS email_otp_challenge (
    account_id UUID PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    -- 每次 request 签发的独立 secret（local_key 加密），不是用户的认证器长期 secret。
    secret_ciphertext BYTEA NOT NULL,
    -- 签发时的 10 分钟窗编号（floor(created_at / 600s)）；成功后记入 used_step，同窗再验拒绝。
    step BIGINT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    last_sent_at TIMESTAMPTZ NOT NULL,
    fail_count INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    used_step BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 认证器 TOTP 防重放：记下已成功使用的 30s 时间窗，同窗再验拒绝。
ALTER TABLE totp_credential ADD COLUMN IF NOT EXISTS last_used_step BIGINT;
