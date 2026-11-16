-- Company mailbox identities. A row does not mean Resend is receiving mail for it.

CREATE TABLE mailboxes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    address citext NOT NULL UNIQUE,
    display_name text NOT NULL,
    provider text NOT NULL DEFAULT 'resend',
    provider_inbox_id text,
    is_enabled boolean NOT NULL DEFAULT false,
    is_receiving_verified boolean NOT NULL DEFAULT false,
    verified_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT mailboxes_display_name_chk CHECK (length(trim(display_name)) > 0),
    CONSTRAINT mailboxes_provider_chk CHECK (length(trim(provider)) > 0)
);

CREATE UNIQUE INDEX mailboxes_provider_inbox_idx
    ON mailboxes (provider, provider_inbox_id)
    WHERE provider_inbox_id IS NOT NULL;

CREATE TRIGGER mailboxes_set_updated_at
BEFORE UPDATE ON mailboxes
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE mailbox_users (
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    can_read boolean NOT NULL DEFAULT true,
    can_send boolean NOT NULL DEFAULT false,
    can_manage boolean NOT NULL DEFAULT false,
    is_default boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (mailbox_id, user_id)
);

CREATE UNIQUE INDEX mailbox_users_one_default_idx
    ON mailbox_users (user_id)
    WHERE is_default;

CREATE TABLE mailbox_settings (
    mailbox_id uuid PRIMARY KEY REFERENCES mailboxes (id) ON DELETE CASCADE,
    signature_id uuid,
    auto_bcc citext,
    retention_days integer,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT mailbox_settings_retention_chk CHECK (retention_days IS NULL OR retention_days > 0)
);

CREATE TRIGGER mailbox_settings_set_updated_at
BEFORE UPDATE ON mailbox_settings
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

ALTER TABLE user_preferences
    ADD CONSTRAINT user_preferences_default_mailbox_fk
    FOREIGN KEY (default_mailbox_id) REFERENCES mailboxes (id) ON DELETE SET NULL;
