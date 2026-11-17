-- A group is a local address. Mail sent to it is delivered to each member mailbox.

CREATE TABLE mailbox_groups (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL,
    address citext NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT mailbox_groups_name_chk CHECK (
        length(trim(name)) > 0 AND length(trim(name)) <= 80
    )
);

CREATE TRIGGER mailbox_groups_set_updated_at
BEFORE UPDATE ON mailbox_groups
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

CREATE TABLE mailbox_group_members (
    group_id uuid NOT NULL REFERENCES mailbox_groups (id) ON DELETE CASCADE,
    mailbox_id uuid NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, mailbox_id)
);

CREATE INDEX mailbox_group_members_mailbox_idx
    ON mailbox_group_members (mailbox_id);
