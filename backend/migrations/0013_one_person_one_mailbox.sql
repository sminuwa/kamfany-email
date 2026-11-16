-- One person, one mailbox. The mailbox address is that person's sign-in address.

DELETE FROM mailbox_users AS grant_row
USING users AS person, mailboxes AS mailbox
WHERE grant_row.user_id = person.id
  AND grant_row.mailbox_id = mailbox.id
  AND person.email <> mailbox.address;

DELETE FROM message_folder_assignments AS assignment
USING folders AS folder, mailboxes AS mailbox
WHERE assignment.folder_id = folder.id
  AND folder.mailbox_id = mailbox.id
  AND NOT EXISTS (
      SELECT 1
      FROM users AS person
      WHERE person.email = mailbox.address
  );

DELETE FROM mailboxes AS mailbox
WHERE NOT EXISTS (
    SELECT 1
    FROM users AS person
    WHERE person.email = mailbox.address
);

INSERT INTO mailboxes (address, display_name, is_enabled)
SELECT person.email, person.display_name, true
FROM users AS person
WHERE NOT EXISTS (
    SELECT 1
    FROM mailboxes AS mailbox
    WHERE mailbox.address = person.email
);

INSERT INTO mailbox_settings (mailbox_id)
SELECT mailbox.id
FROM mailboxes AS mailbox
WHERE NOT EXISTS (
    SELECT 1
    FROM mailbox_settings AS settings
    WHERE settings.mailbox_id = mailbox.id
);

INSERT INTO folders (mailbox_id, system_key, name, sort_order)
SELECT mailbox.id, folder.key, folder.name, folder.sort_order
FROM mailboxes AS mailbox
CROSS JOIN (
    VALUES
        ('inbox', 'Inbox', 0),
        ('starred', 'Starred', 1),
        ('snoozed', 'Snoozed', 2),
        ('sent', 'Sent', 3),
        ('drafts', 'Drafts', 4),
        ('archive', 'Archive', 5),
        ('spam', 'Spam', 6),
        ('trash', 'Trash', 7)
) AS folder (key, name, sort_order)
WHERE NOT EXISTS (
    SELECT 1
    FROM folders AS existing
    WHERE existing.mailbox_id = mailbox.id
      AND existing.system_key = folder.key
);

INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage, is_default)
SELECT mailbox.id, person.id, true, true, true, true
FROM users AS person
JOIN mailboxes AS mailbox ON mailbox.address = person.email
ON CONFLICT (mailbox_id, user_id) DO UPDATE
SET can_read = true,
    can_send = true,
    can_manage = true,
    is_default = true;

UPDATE user_preferences AS preference
SET default_mailbox_id = mailbox.id
FROM users AS person
JOIN mailboxes AS mailbox ON mailbox.address = person.email
WHERE preference.user_id = person.id;

CREATE UNIQUE INDEX mailbox_users_one_person_idx ON mailbox_users (user_id);

CREATE UNIQUE INDEX mailbox_users_one_mailbox_idx ON mailbox_users (mailbox_id);
