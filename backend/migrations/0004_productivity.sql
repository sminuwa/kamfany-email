-- Contacts, signatures, drafts, notifications, and inbound provider events.

CREATE TABLE contacts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    full_name text NOT NULL,
    email citext,
    phone text,
    company text,
    avatar_storage_key text,
    notes text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT contacts_name_chk CHECK (length(trim(full_name)) > 0)
);

CREATE INDEX contacts_owner_idx ON contacts (owner_user_id);
CREATE INDEX contacts_owner_email_idx ON contacts (owner_user_id, email);

CREATE TRIGGER contacts_set_updated_at
BEFORE UPDATE ON contacts
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE email_signatures (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    mailbox_id uuid REFERENCES mailboxes (id) ON DELETE CASCADE,
    name text NOT NULL,
    body_html text NOT NULL DEFAULT '',
    is_default boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT email_signatures_name_chk CHECK (length(trim(name)) > 0)
);

CREATE UNIQUE INDEX email_signatures_one_default_idx
    ON email_signatures (user_id, COALESCE(mailbox_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE is_default;

CREATE TRIGGER email_signatures_set_updated_at
BEFORE UPDATE ON email_signatures
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

ALTER TABLE mailbox_settings
    ADD CONSTRAINT mailbox_settings_signature_fk
    FOREIGN KEY (signature_id) REFERENCES email_signatures (id) ON DELETE SET NULL;

CREATE TABLE drafts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    author_user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    in_reply_to_message_id uuid REFERENCES messages (id) ON DELETE SET NULL,
    to_recipients jsonb NOT NULL DEFAULT '[]'::jsonb,
    cc_recipients jsonb NOT NULL DEFAULT '[]'::jsonb,
    bcc_recipients jsonb NOT NULL DEFAULT '[]'::jsonb,
    subject text NOT NULL DEFAULT '',
    body_html text NOT NULL DEFAULT '',
    body_text text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX drafts_author_idx ON drafts (author_user_id, updated_at DESC);
CREATE INDEX drafts_mailbox_idx ON drafts (mailbox_id, updated_at DESC);

CREATE TRIGGER drafts_set_updated_at
BEFORE UPDATE ON drafts
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE notifications (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    mailbox_id uuid REFERENCES mailboxes (id) ON DELETE CASCADE,
    kind text NOT NULL,
    title text NOT NULL,
    body text NOT NULL DEFAULT '',
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    read_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX notifications_user_unread_idx
    ON notifications (user_id, created_at DESC)
    WHERE read_at IS NULL;

CREATE TABLE webhook_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    provider text NOT NULL,
    provider_event_id text NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    status text NOT NULL DEFAULT 'received',
    attempts integer NOT NULL DEFAULT 0,
    last_error text,
    received_at timestamptz NOT NULL DEFAULT now(),
    processed_at timestamptz,
    CONSTRAINT webhook_events_status_chk CHECK (status IN ('received', 'processing', 'processed', 'failed')),
    CONSTRAINT webhook_events_attempts_chk CHECK (attempts >= 0),
    CONSTRAINT webhook_events_provider_event_chk CHECK (
        length(trim(provider)) > 0 AND length(trim(provider_event_id)) > 0
    ),
    UNIQUE (provider, provider_event_id)
);

CREATE INDEX webhook_events_status_idx
    ON webhook_events (status, received_at);
