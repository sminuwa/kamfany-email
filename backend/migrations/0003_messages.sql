-- Mail organization. Starred and snoozed are message flags, not exclusive folders.
-- Folder rows for those keys may exist so the desk can address them. Do not
-- assign a message only to starred or snoozed.

CREATE TABLE folders (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    parent_id uuid REFERENCES folders (id) ON DELETE CASCADE,
    system_key text,
    name text NOT NULL,
    color text,
    sort_order integer NOT NULL DEFAULT 0,
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT folders_system_key_chk CHECK (
        system_key IS NULL
        OR system_key IN ('inbox', 'starred', 'snoozed', 'sent', 'drafts', 'archive', 'spam', 'trash')
    ),
    CONSTRAINT folders_name_chk CHECK (length(trim(name)) > 0)
);

CREATE UNIQUE INDEX folders_system_key_idx
    ON folders (mailbox_id, system_key)
    WHERE system_key IS NOT NULL;

CREATE UNIQUE INDEX folders_sibling_name_idx
    ON folders (mailbox_id, COALESCE(parent_id, '00000000-0000-0000-0000-000000000000'::uuid), lower(name));

CREATE TRIGGER folders_set_updated_at
BEFORE UPDATE ON folders
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE OR REPLACE FUNCTION folders_same_mailbox()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    parent_mailbox uuid;
BEGIN
    IF NEW.parent_id IS NULL THEN
        RETURN NEW;
    END IF;
    IF NEW.parent_id = NEW.id THEN
        RAISE EXCEPTION 'folder cannot be its own parent';
    END IF;
    SELECT mailbox_id INTO parent_mailbox FROM folders WHERE id = NEW.parent_id;
    IF parent_mailbox IS DISTINCT FROM NEW.mailbox_id THEN
        RAISE EXCEPTION 'folder parent must belong to the same mailbox';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER folders_same_mailbox
BEFORE INSERT OR UPDATE ON folders
FOR EACH ROW
EXECUTE FUNCTION folders_same_mailbox();

CREATE TABLE labels (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    name text NOT NULL,
    color text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT labels_name_chk CHECK (length(trim(name)) > 0)
);

CREATE UNIQUE INDEX labels_mailbox_name_idx
    ON labels (mailbox_id, lower(name));

CREATE TRIGGER labels_set_updated_at
BEFORE UPDATE ON labels
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE threads (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    subject text NOT NULL DEFAULT '',
    last_message_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX threads_mailbox_last_idx
    ON threads (mailbox_id, last_message_at DESC);

CREATE TRIGGER threads_set_updated_at
BEFORE UPDATE ON threads
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE messages (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    thread_id uuid REFERENCES threads (id) ON DELETE SET NULL,
    provider text NOT NULL,
    provider_message_id text,
    direction text NOT NULL,
    status text NOT NULL,
    from_address citext NOT NULL,
    from_name text,
    subject text NOT NULL DEFAULT '',
    preview text,
    body_text text,
    body_html text,
    body_storage_key text,
    received_at timestamptz,
    sent_at timestamptz,
    is_read boolean NOT NULL DEFAULT false,
    is_starred boolean NOT NULL DEFAULT false,
    is_important boolean NOT NULL DEFAULT false,
    snoozed_until timestamptz,
    has_attachments boolean NOT NULL DEFAULT false,
    message_id_header text,
    in_reply_to text,
    references_header text,
    raw_storage_key text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT messages_direction_chk CHECK (direction IN ('inbound', 'outbound')),
    CONSTRAINT messages_status_chk CHECK (
        status IN (
            'queued',
            'sending',
            'accepted',
            'delivered',
            'bounced',
            'failed',
            'received',
            'draft'
        )
    ),
    CONSTRAINT messages_provider_chk CHECK (length(trim(provider)) > 0)
);

CREATE UNIQUE INDEX messages_provider_id_idx
    ON messages (provider, provider_message_id)
    WHERE provider_message_id IS NOT NULL;

CREATE INDEX messages_mailbox_created_idx
    ON messages (mailbox_id, created_at DESC);

CREATE INDEX messages_mailbox_status_idx
    ON messages (mailbox_id, status, created_at DESC);

CREATE INDEX messages_mailbox_unread_idx
    ON messages (mailbox_id, created_at DESC)
    WHERE is_read = false;

CREATE INDEX messages_thread_idx
    ON messages (thread_id);

CREATE INDEX messages_mailbox_starred_idx
    ON messages (mailbox_id)
    WHERE is_starred;

CREATE INDEX messages_snoozed_idx
    ON messages (mailbox_id, snoozed_until)
    WHERE snoozed_until IS NOT NULL;

ALTER TABLE messages
    ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(subject, '')), 'A')
        || setweight(to_tsvector('english', coalesce(from_name, '')), 'A')
        || setweight(to_tsvector('english', coalesce(from_address::text, '')), 'A')
        || setweight(to_tsvector('english', coalesce(body_text, '')), 'B')
    ) STORED;

CREATE INDEX messages_search_idx
    ON messages USING gin (search_vector);

CREATE TRIGGER messages_set_updated_at
BEFORE UPDATE ON messages
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE recipients (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id uuid NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    kind text NOT NULL,
    address citext NOT NULL,
    name text,
    CONSTRAINT recipients_kind_chk CHECK (kind IN ('to', 'cc', 'bcc'))
);

CREATE INDEX recipients_message_idx ON recipients (message_id);
CREATE INDEX recipients_address_idx ON recipients (address);

CREATE TABLE message_labels (
    message_id uuid NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    label_id uuid NOT NULL REFERENCES labels (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, label_id)
);

CREATE OR REPLACE FUNCTION message_label_same_mailbox()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    label_mailbox uuid;
    message_mailbox uuid;
BEGIN
    SELECT mailbox_id INTO label_mailbox FROM labels WHERE id = NEW.label_id;
    SELECT mailbox_id INTO message_mailbox FROM messages WHERE id = NEW.message_id;
    IF label_mailbox IS DISTINCT FROM message_mailbox THEN
        RAISE EXCEPTION 'label and message must belong to the same mailbox';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER message_labels_same_mailbox
BEFORE INSERT OR UPDATE ON message_labels
FOR EACH ROW
EXECUTE FUNCTION message_label_same_mailbox();

-- One organizational folder per message. Labels carry overlapping classification.
CREATE TABLE message_folder_assignments (
    message_id uuid PRIMARY KEY REFERENCES messages (id) ON DELETE CASCADE,
    folder_id uuid NOT NULL REFERENCES folders (id) ON DELETE RESTRICT,
    assigned_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX message_folder_folder_idx
    ON message_folder_assignments (folder_id);

CREATE OR REPLACE FUNCTION message_folder_same_mailbox()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    folder_mailbox uuid;
    message_mailbox uuid;
BEGIN
    SELECT mailbox_id INTO folder_mailbox FROM folders WHERE id = NEW.folder_id;
    SELECT mailbox_id INTO message_mailbox FROM messages WHERE id = NEW.message_id;
    IF folder_mailbox IS DISTINCT FROM message_mailbox THEN
        RAISE EXCEPTION 'folder and message must belong to the same mailbox';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER message_folder_same_mailbox
BEFORE INSERT OR UPDATE ON message_folder_assignments
FOR EACH ROW
EXECUTE FUNCTION message_folder_same_mailbox();

CREATE TABLE attachments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id uuid NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    filename text NOT NULL,
    content_type text NOT NULL DEFAULT 'application/octet-stream',
    size_bytes bigint NOT NULL,
    storage_key text NOT NULL UNIQUE,
    checksum_sha256 text,
    scan_status text NOT NULL DEFAULT 'pending',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT attachments_size_chk CHECK (size_bytes >= 0),
    CONSTRAINT attachments_filename_chk CHECK (length(trim(filename)) > 0),
    CONSTRAINT attachments_scan_chk CHECK (scan_status IN ('pending', 'clean', 'rejected', 'failed'))
);

CREATE INDEX attachments_message_idx ON attachments (message_id);
