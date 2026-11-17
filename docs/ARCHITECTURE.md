# Architecture

This is a company mail desk that rides on Resend. One deployment serves one organization. The product name, company, public address, logo, and sending domain come from the API environment, and the desk reads them from `GET /api/v1/brand`. Staff send, read, and organize mail. The product has to feel as capable as a modern webmail client, with its own identity, and it must not pretend the email provider is a traditional mailbox.

## Shape

```text
Vue desk  -->  Rust API  -->  Email provider (Resend first)
                 |
                 +--> PostgreSQL
                 +--> Redis, when jobs and presence need it
                 +--> object storage (local directory, or S3 when configured)
```

The Vue app never calls Resend. API keys, webhook secrets, and provider payloads stay behind the Rust process. Another provider can be added later by implementing the same internal boundary. Resend types must not leak into handlers, stores, or Vue components.

The provider boundary is the Resend client. Handlers and the Vue desk do not see Resend types.

## Directory

```text
kamfany-mail/
├── frontend/
│   ├── src/components/   layout, common, and later mail, compose, contacts, settings
│   ├── src/layouts/      MailLayout, AuthLayout
│   ├── src/pages/        mail desk, account
│   ├── src/router/
│   ├── src/stores/       preferences, ui, auth, service
│   ├── src/services/     Axios client and health
│   ├── src/composables/
│   ├── src/styles/       design tokens
│   └── src/utils/
└── backend/
    ├── migrations/       initial PostgreSQL schema
    └── src/
        ├── api/          router, errors, health
        ├── config.rs
        ├── state.rs
        ├── database/
        └── providers/    boundary only, until Phase 4
```

Feature modules (`auth`, `mailboxes`, `messages`, `jobs`, `storage`, and the Resend client) are added with the phase that implements them, not as empty shells.

## Crates and packages

Installed:

- Rust: tokio, axum, sqlx, serde, serde_json, tracing, tracing-subscriber, tower, tower-http, thiserror, anyhow, dotenvy
- Frontend: vue, vue-router, pinia, axios, @vueuse/core, tailwindcss, @fontsource/public-sans, @lucide/vue, dayjs

`lucide-vue-next` is deprecated and installs as a redirect to `@lucide/vue`. The desk uses `@lucide/vue`, which is the current Lucide Vue package.

Also installed for the current desk: reqwest, argon2, uuid, chrono, ammonia, object_store, and TipTap. Redis and OpenAPI generation are still later.

## Database

PostgreSQL, UUID primary keys, SQLx migrations. The binary command `migrate` applies `backend/migrations`.

| Area | Tables |
| --- | --- |
| Identity | users, roles, permissions, user_roles, role_permissions, sessions, user_preferences, audit_logs |
| Mailboxes | mailboxes, mailbox_users, mailbox_settings |
| Mail | folders, labels, threads, messages, recipients, message_labels, message_folder_assignments, attachments |
| Other | contacts, drafts, email_signatures, notifications, webhook_events |

Messages keep an internal id and a separate `(provider, provider_message_id)`. Bodies can live in `body_text` / `body_html` or move to object storage via `body_storage_key`. Attachment bytes are not stored in PostgreSQL. `attachments.storage_key` points at an object. A row belongs to a draft or a message, not both. `scan_status` is `pending` until `CLAMAV_ADDRESS` points at clamd. A real `INSTREAM` reply can mark the file `clean` or `rejected`. A scanner error marks it `failed`. Nothing is marked clean without that reply.

A message has one organizational folder. Labels are many-to-many. Starred and snoozed are flags (`is_starred`, `snoozed_until`), not exclusive folders. Delivery status is `queued`, `sending`, `accepted`, `delivered`, `bounced`, or `failed`. Provider acceptance is not delivery.

Search uses a generated `tsvector` on subject, sender, and plain text, plus the relational filters in `messages::search`. HTML is not indexed. The search service is separate from the folder listing so it can change later. A query returns the latest 50 matches the caller is allowed to read.

Creating a mailbox row does not create a receiving address. A new mailbox is enabled. `is_receiving_verified` stays false until the provider route is confirmed.

Seeded roles are owner, administrator, and member. Mailbox grants (`can_read`, `can_send`, `can_manage`) are separate from platform roles.

## Resend

Resend is a sending and receiving API, not IMAP or POP3. The client lives in `backend/src/providers` and speaks only the operations confirmed in the current docs: send email, list domains, list inboxes, and retrieve a received email. Conversation APIs are not used.

- Outbound mail is `POST /emails` with an idempotency key. The stored status becomes `accepted` when Resend returns an id. `email.delivered`, `email.bounced`, and `email.failed` are the later status changes. A complaint is kept on the webhook row and does not invent a delivery state.
- Inbound mail is an `email.received` webhook. The body is not in the webhook. A job calls `GET /emails/receiving/{id}` and stores the message for each local mailbox whose address matches. Attachment bytes are downloaded from the provider’s signed URL, without the API key, and stored as objects. A missing attachment list does not fail the message.
- `is_receiving_verified` becomes true only when Resend lists that inbox address or a received message is stored for it. A verified domain is not enough.
- Webhook signatures use the Svix headers and the raw body. Unsigned requests are refused. Duplicate `svix-id` values are ignored.
- The queue is the `webhook_events` table, claimed with `FOR UPDATE SKIP LOCKED`, with backoff. It is durable in PostgreSQL. Redis is not required for this queue. The provider status for `mailbox.manage` reports how many events are received, processing, or failed, and the age of the oldest unprocessed event.

Reply and forward send through `POST /emails` as well. The request includes `In-Reply-To` and `References` when the source message has a message id. A reply stays on the source thread. A forward starts a new thread and still carries those headers. `POST /api/v1/mailboxes/{id}/send` remains the new-message API.

## Production controls

Sign-in, password reset, webhook intake, sending, and uploads charge a PostgreSQL `rate_limits` row. The sign-in budget is the connecting address, taken from `X-Forwarded-For` only when that value is an IP. Nginx overwrites that header with `$remote_addr`. The account lock of five failures remains separate.

`GET /api/v1/audit` is limited to `audit.read`. The list does not include the metadata column. Contact deletion and permanent message deletion write audit rows, as sign-in, sending, and mailbox changes already did.

API responses set `nosniff`, `DENY` framing, and `no-referrer`. The HTML content security policy lives in `deploy/nginx/mail.kamfany.com.conf` because the API returns JSON. `deploy/systemd` holds the API service and a daily `pg_dump` timer. The dump stays on the server for 14 days. Copying it off the server is a separate step. Those units are not installed by the app.

## External dependencies and limits

- PostgreSQL is required to migrate and to report ready.
- Redis is not required for the inbound queue. `webhook_events` is the durable queue.
- Attachment bytes go to `STORAGE_DIR` (default `storage/`) unless `S3_BUCKET` is set. Downloads are session-scoped. There are no public object URLs.
- A Resend account, a verified domain, an API key, and a publicly reachable webhook URL are required before real mail can flow. None of those secrets belong in the repo.
- The app cannot import an existing IMAP mailbox.
- The app cannot assume inbox, conversation, or reply APIs that the current Resend product does not offer.
- Full-text search currently indexes `body_text`. Very large messages may later need a truncated search column.

## API

Base path `/api/v1`. Success responses use `{ "data": ... }`. Errors use `{ "error": { "code", "message" } }`. Database failures are logged and not returned to the client.

Phase 1:

```http
GET /api/v1/health
GET /api/v1/ready
```

`/health` is liveness and returns 200 even when PostgreSQL is down. `/ready` returns 503 until the database answers. Unknown routes return the JSON error envelope.

OpenAPI generation starts when the resource APIs exist, so the spec is not a health-only stand-in for the mail API.

## Authentication

Passwords are Argon2 hashes. Sessions are random tokens stored as SHA-256 hashes, sent in the HttpOnly `kamfany_session` cookie (`SameSite=Lax`, seven days). `Secure` is set only when `APP_ENV=production`. Changing a password revokes the other sessions. Five failed sign-ins lock the account for 15 minutes. Unknown addresses, bad passwords, inactive users, and locked users all return the same 401.

A mailbox id from the browser is loaded only when the caller has a grant, or `mailbox.manage` for administration. A manager without a grant can edit the mailbox and still cannot read its mail. `is_receiving_verified` is not accepted from the client. Two-factor columns on `users` are unused.

In development, `BOOTSTRAP_ADMIN_EMAIL` and `BOOTSTRAP_ADMIN_PASSWORD` create the owner and the five company mailboxes if they are missing. The password on an existing account is left alone. This does not run in test or production.

## Visual system

The desk is an all-day operating surface. The navigation rail is a deep lagoon field. The work area is a cool mist, not a warm paper page. Brass is reserved for the mark, the active folder icon, and the compose hover. Public Sans is the UI face. Light, dark, and system themes share one token file: `frontend/src/styles/tokens.css`.

## Phases

The desk lists stored messages for the open folder. HTML is sanitized and shown in a sandboxed iframe. Reply, reply all, forward, and a new message send sanitized HTML through the existing Resend path when the mailbox can send. Drafts, signatures, and attachments are part of that composer. Contacts, server search, and inbound notices are part of the desk. A live socket updates the open inbox for browsers connected to this API process. Rate limits, audit review, ClamAV scanning when configured, and the Nginx and systemd units are part of the production shape. Sample records remain in `frontend/src/mocks/mailbox.ts` for tests. They are not shown on the desk.

Future multi-tenant features stay out until the first internal release is stable.
