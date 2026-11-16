-- A received message notifies each reader once.

ALTER TABLE notifications
    ADD COLUMN message_id uuid REFERENCES messages (id) ON DELETE CASCADE;

CREATE UNIQUE INDEX notifications_user_message_idx
    ON notifications (user_id, message_id)
    WHERE message_id IS NOT NULL;

CREATE UNIQUE INDEX contacts_owner_email_unique
    ON contacts (owner_user_id, email)
    WHERE email IS NOT NULL;
