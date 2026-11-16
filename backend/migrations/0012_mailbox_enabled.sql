-- A registered mailbox is enabled. Receiving stays unverified until the provider route is confirmed.

ALTER TABLE mailboxes
    ALTER COLUMN is_enabled SET DEFAULT true;

UPDATE mailboxes
SET is_enabled = true
WHERE is_enabled = false;
