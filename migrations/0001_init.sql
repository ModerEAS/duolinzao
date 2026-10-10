-- 多邻灶一期。业务数字不写进迁移。

CREATE TABLE account (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    password_hash TEXT,
    must_change_password BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE invite_code (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code_hash TEXT NOT NULL UNIQUE,
    created_by UUID NOT NULL REFERENCES account (id),
    used_by UUID REFERENCES account (id),
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE canteen (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE role_assignment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('employee', 'canteen', 'admin', 'super_admin')),
    canteen_ids UUID[] NOT NULL DEFAULT '{}',
    source_label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT role_scope_chk CHECK (
        (role = 'super_admin' AND cardinality(canteen_ids) = 0)
        OR (role = 'employee' AND cardinality(canteen_ids) = 0)
        OR (role IN ('canteen', 'admin') AND cardinality(canteen_ids) >= 1)
    )
);

CREATE INDEX role_assignment_account ON role_assignment (account_id);

CREATE TABLE canteen_member (
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    canteen_id UUID NOT NULL REFERENCES canteen (id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, canteen_id)
);

CREATE TABLE canteen_staff (
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    canteen_id UUID NOT NULL REFERENCES canteen (id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, canteen_id)
);

CREATE TABLE device (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    friendly_name TEXT NOT NULL,
    user_agent TEXT NOT NULL,
    ip TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX device_account ON device (account_id);

CREATE TABLE session (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES device (id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX session_account ON session (account_id);

CREATE TABLE passkey (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE,
    passkey_json JSONB NOT NULL,
    friendly_name TEXT,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX passkey_account ON passkey (account_id);

CREATE TABLE totp_credential (
    account_id UUID PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    secret_ciphertext BYTEA NOT NULL,
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE magic_link (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE login_approval (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'allowed', 'denied')),
    user_agent TEXT NOT NULL,
    friendly_name TEXT NOT NULL,
    ip TEXT NOT NULL,
    poll_token_hash TEXT NOT NULL UNIQUE,
    granted_token_ciphertext BYTEA,
    granted_session_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at TIMESTAMPTZ,
    decided_by UUID
);

CREATE INDEX login_approval_account_status ON login_approval (account_id, status);

CREATE TABLE login_attempt (
    id BIGSERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX login_attempt_key_time ON login_attempt (key, created_at);

CREATE TABLE shift_plan (
    account_id UUID PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('work2_rest4', 'custom')),
    anchor_date DATE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE meal_template (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL UNIQUE REFERENCES account (id) ON DELETE CASCADE,
    dsl JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE meal_day_lock (
    meal_date DATE PRIMARY KEY,
    locked_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE meal_order (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    meal_date DATE NOT NULL,
    slot TEXT NOT NULL CHECK (slot IN ('breakfast', 'lunch', 'dinner')),
    canteen_id UUID REFERENCES canteen (id),
    ordering BOOLEAN NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('dsl', 'manual', 'admin')),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_at TIMESTAMPTZ,
    UNIQUE (account_id, meal_date, slot)
);

CREATE INDEX meal_order_canteen_date ON meal_order (canteen_id, meal_date);

CREATE TABLE dish (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canteen_id UUID NOT NULL REFERENCES canteen (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('菜', '饭', '汤')),
    taste_tags TEXT[] NOT NULL DEFAULT '{}',
    image_path TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX dish_canteen ON dish (canteen_id);

CREATE TABLE menu_pick (
    canteen_id UUID NOT NULL REFERENCES canteen (id) ON DELETE CASCADE,
    meal_date DATE NOT NULL,
    slot TEXT NOT NULL CHECK (slot IN ('breakfast', 'lunch', 'dinner')),
    dish_id UUID NOT NULL REFERENCES dish (id) ON DELETE CASCADE,
    PRIMARY KEY (canteen_id, meal_date, slot, dish_id)
);

CREATE TABLE score (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    meal_date DATE NOT NULL,
    slot TEXT NOT NULL CHECK (slot IN ('breakfast', 'lunch', 'dinner')),
    dish_id UUID NOT NULL REFERENCES dish (id),
    value SMALLINT NOT NULL CHECK (value BETWEEN -2 AND 2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (account_id, meal_date, slot, dish_id)
);

CREATE INDEX score_dish ON score (dish_id);

CREATE TABLE comment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    score_id UUID NOT NULL UNIQUE REFERENCES score (id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 300),
    anonymous BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE preference (
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    dish_id UUID NOT NULL REFERENCES dish (id) ON DELETE CASCADE,
    mean DOUBLE PRECISION NOT NULL,
    sample_count INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, dish_id)
);

CREATE TABLE notification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX notification_account ON notification (account_id, created_at DESC);

CREATE TABLE llm_channel (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('local_script', 'openai_chat', 'openai_responses', 'anthropic')),
    base_url TEXT,
    model TEXT,
    key_ciphertext BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_id UUID,
    actor_username TEXT,
    role TEXT,
    scope TEXT NOT NULL,
    action TEXT NOT NULL,
    object_type TEXT,
    object_id TEXT,
    summary JSONB NOT NULL DEFAULT '{}'::jsonb,
    result TEXT NOT NULL
);

CREATE INDEX audit_log_created ON audit_log (created_at DESC);

CREATE OR REPLACE FUNCTION prevent_young_audit_delete()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.created_at > (now() - interval '6 months') THEN
        RAISE EXCEPTION 'audit_retention_violation'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER audit_no_early_delete
BEFORE DELETE ON audit_log
FOR EACH ROW
EXECUTE FUNCTION prevent_young_audit_delete();

CREATE OR REPLACE FUNCTION purge_audit_older_than_6_months()
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    n BIGINT;
BEGIN
    DELETE FROM audit_log
    WHERE created_at <= (now() - interval '6 months');
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END;
$$;

CREATE TABLE app_setting (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
