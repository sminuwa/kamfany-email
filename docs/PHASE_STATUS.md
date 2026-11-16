# Phase status

**Current phase:** 8 — Security and production  
**State:** Built. Not deployed. Waiting for approval before any server install, DNS change, or MX change.

## Delivered

Sign-in, password reset, webhook intake, sending, and uploads are counted in PostgreSQL. A full bucket returns 429. The sign-in budget uses the connecting address. The existing account lock still applies to a password.

`GET /api/v1/audit` lists recent actions for someone with `audit.read`. A member receives 403. The list does not include metadata. Deleting a contact or permanently deleting a message writes an audit row.

Attachments are scanned with ClamAV `INSTREAM` when `CLAMAV_ADDRESS` is set. With no scanner, files stay `pending` and can still be sent, so a webhook test is not blocked. A rejected file cannot be sent or downloaded. A failed scan blocks sending when a scanner is configured. Inbound mail is still stored if the scanner is down, and that file is marked `failed`.

The provider panel shows the webhook queue and whether scanning is configured. The account page lists the audit log for an owner or administrator.

`deploy/` contains the Nginx site, the API systemd service, a `pg_dump` script, and a daily timer. Dumps are kept on the server for 14 days. They are not uploaded anywhere. Nothing in this phase was installed on a host, and `kamfany.com` MX was not changed.

## API

```http
GET /api/v1/audit?limit=
```

`GET /api/v1/provider` now also returns `scanner`, `queue_received`, `queue_processing`, `queue_failed`, and `queue_oldest_seconds`.

Attachment objects include `scan_status`.

## Intentionally not in this phase

- Installing Nginx, systemd, or certificates.
- Pointing `kamfany.com` MX at Resend, or enabling receiving.
- A webhook signing secret. Inbound events are still refused until it is set.
- Marking a file clean when ClamAV is not configured.
- Copying database dumps off the server.
- Snooze. Nothing in the desk sets `snoozed_until`.
- Password reset by email.
- Notices to browsers connected to another API process. The rows are shared. The socket is local to this process.
- Search inside drafts.
- OpenAPI generation.

## Checks

- `cargo test --all-targets`: 46 passed.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `npm test`: 37 passed.
- `npm run build`: typecheck and production build succeeded.
- Desk check on `http://localhost:5173`: the account page shows the queue, “Malware scanning is not configured”, and `auth.login` in the audit log. Search still finds “Testing outbout”. An uploaded file is labelled “Not scanned”, and that draft was discarded. No live message was sent.
