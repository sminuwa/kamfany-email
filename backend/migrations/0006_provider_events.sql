-- Durable inbound and delivery work. The row is the queue: the webhook
-- request only verifies the signature and inserts it.

CREATE TABLE webhook_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    provider text NOT NULL,
    provider_event_id text NOT NULL,
    event_type text NOT NULL,
    payload jsonb NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    attempts integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    last_error text,
    processed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT webhook_events_provider_chk CHECK (length(trim(provider)) > 0),
    CONSTRAINT webhook_events_event_chk CHECK (length(trim(event_type)) > 0),
    CONSTRAINT webhook_events_status_chk CHECK (
        status IN ('pending', 'processing', 'processed', 'failed')
    ),
    CONSTRAINT webhook_events_attempts_chk CHECK (attempts >= 0),
    UNIQUE (provider, provider_event_id)
);

CREATE INDEX webhook_events_due_idx
    ON webhook_events (next_attempt_at)
    WHERE status IN ('pending', 'processing');

CREATE TRIGGER webhook_events_set_updated_at
BEFORE UPDATE ON webhook_events
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- One provider email can be stored for each local mailbox it was addressed to.
DROP INDEX messages_provider_id_idx;

CREATE UNIQUE INDEX messages_provider_mailbox_idx
    ON messages (mailbox_id, provider, provider_message_id)
    WHERE provider_message_id IS NOT NULL;
