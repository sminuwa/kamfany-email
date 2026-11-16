-- System folders for mailboxes that already exist, one thread per stored
-- message, and a folder assignment for any message that does not have one.

INSERT INTO folders (mailbox_id, system_key, name, sort_order)
SELECT mailboxes.id, system.system_key, system.name, system.sort_order
FROM mailboxes
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
) AS system(system_key, name, sort_order)
ON CONFLICT (mailbox_id, system_key) WHERE system_key IS NOT NULL DO NOTHING;

INSERT INTO threads (id, mailbox_id, subject, last_message_at)
SELECT messages.id,
       messages.mailbox_id,
       messages.subject,
       COALESCE(messages.received_at, messages.sent_at, messages.created_at)
FROM messages
WHERE messages.thread_id IS NULL;

UPDATE messages
SET thread_id = id
WHERE thread_id IS NULL;

INSERT INTO message_folder_assignments (message_id, folder_id)
SELECT messages.id, folders.id
FROM messages
JOIN folders
    ON folders.mailbox_id = messages.mailbox_id
   AND folders.system_key = CASE
        WHEN messages.direction = 'outbound' THEN 'sent'
        ELSE 'inbox'
    END
WHERE NOT EXISTS (
    SELECT 1
    FROM message_folder_assignments existing
    WHERE existing.message_id = messages.id
);
