-- Drafts can hold files before a message exists. Received files keep the
-- provider attachment id so a retried inbound job does not store them twice.

ALTER TABLE attachments
    ALTER COLUMN message_id DROP NOT NULL;

ALTER TABLE attachments
    ADD COLUMN draft_id uuid REFERENCES drafts (id) ON DELETE CASCADE,
    ADD COLUMN provider_attachment_id text;

ALTER TABLE attachments
    ADD CONSTRAINT attachments_owner_chk CHECK (
        (message_id IS NOT NULL AND draft_id IS NULL)
        OR (message_id IS NULL AND draft_id IS NOT NULL)
    );

CREATE INDEX attachments_draft_idx ON attachments (draft_id);

CREATE UNIQUE INDEX attachments_provider_idx
    ON attachments (message_id, provider_attachment_id)
    WHERE provider_attachment_id IS NOT NULL;

ALTER TABLE drafts
    ADD COLUMN compose_kind text NOT NULL DEFAULT 'new';

ALTER TABLE drafts
    ADD CONSTRAINT drafts_kind_chk CHECK (
        compose_kind IN ('new', 'reply', 'reply-all', 'forward')
    );
