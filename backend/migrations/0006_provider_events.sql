-- Durable inbound and delivery work. The row is the queue: the webhook
-- request only verifies the signature and inserts it.
--
-- Migration 4 created the first version of webhook_events. This migration
-- brings that table up to the worker shape. A table that already has
-- next_attempt_at is left in place.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'webhook_events'
          AND column_name = 'next_attempt_at'
    ) THEN
        ALTER TABLE webhook_events
            ADD COLUMN next_attempt_at timestamptz,
            ADD COLUMN created_at timestamptz,
            ADD COLUMN updated_at timestamptz;

        UPDATE webhook_events
        SET next_attempt_at = COALESCE(received_at, now()),
            created_at = COALESCE(received_at, now()),
            updated_at = now();

        ALTER TABLE webhook_events
            ALTER COLUMN next_attempt_at SET DEFAULT now(),
            ALTER COLUMN next_attempt_at SET NOT NULL,
            ALTER COLUMN created_at SET DEFAULT now(),
            ALTER COLUMN created_at SET NOT NULL,
            ALTER COLUMN updated_at SET DEFAULT now(),
            ALTER COLUMN updated_at SET NOT NULL,
            ALTER COLUMN status SET DEFAULT 'pending';

        UPDATE webhook_events SET status = 'pending' WHERE status = 'received';

        ALTER TABLE webhook_events DROP CONSTRAINT webhook_events_status_chk;
        ALTER TABLE webhook_events ADD CONSTRAINT webhook_events_status_chk CHECK (
            status IN ('pending', 'processing', 'processed', 'failed')
        );

        ALTER TABLE webhook_events DROP CONSTRAINT webhook_events_provider_event_chk;
        ALTER TABLE webhook_events
            ADD CONSTRAINT webhook_events_provider_chk CHECK (length(trim(provider)) > 0),
            ADD CONSTRAINT webhook_events_event_chk CHECK (length(trim(event_type)) > 0);

        DROP INDEX webhook_events_status_idx;
        ALTER TABLE webhook_events DROP COLUMN received_at;

        CREATE INDEX webhook_events_due_idx
            ON webhook_events (next_attempt_at)
            WHERE status IN ('pending', 'processing');

        CREATE TRIGGER webhook_events_set_updated_at
        BEFORE UPDATE ON webhook_events
        FOR EACH ROW
        EXECUTE FUNCTION set_updated_at();
    END IF;
END $$;

-- One provider email can be stored for each local mailbox it was addressed to.
DROP INDEX IF EXISTS messages_provider_id_idx;

CREATE UNIQUE INDEX IF NOT EXISTS messages_provider_mailbox_idx
    ON messages (mailbox_id, provider, provider_message_id)
    WHERE provider_message_id IS NOT NULL;
