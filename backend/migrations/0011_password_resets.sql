-- One-time password reset links. Only the hash of the token is stored.

CREATE TABLE password_resets (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT password_resets_token_hash_chk CHECK (length(token_hash) = 64)
);

CREATE INDEX password_resets_user_open_idx
    ON password_resets (user_id)
    WHERE used_at IS NULL;
