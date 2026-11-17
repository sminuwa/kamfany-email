-- A group is a named list of mailboxes, not an address.

ALTER TABLE mailbox_groups DROP COLUMN address;

WITH ranked AS (
    SELECT id,
           name,
           row_number() OVER (PARTITION BY lower(name) ORDER BY created_at, id) AS n
    FROM mailbox_groups
)
UPDATE mailbox_groups AS groups
SET name = left(ranked.name, 74) || ' ' || ranked.n::text
FROM ranked
WHERE groups.id = ranked.id
  AND ranked.n > 1;

CREATE UNIQUE INDEX mailbox_groups_name_idx ON mailbox_groups (lower(name));
