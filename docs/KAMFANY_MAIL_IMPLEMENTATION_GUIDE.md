> Master implementation guide for KAMFANY Mail. Follow it for the life of the project.
> Build one phase at a time. The active phase is recorded in [PHASE_STATUS.md](./PHASE_STATUS.md).
> Architecture notes for the current build live in [ARCHITECTURE.md](./ARCHITECTURE.md).

# KAMFANY Mail
## Master AI Development Prompt — Vue 3 + Rust + Resend

**Company:** IT KAMFANY LIMITED  
**Product:** KAMFANY Mail  
**Web application:** `mail.kamfany.com`  
**Frontend:** Vue 3 + TypeScript  
**Backend:** Rust + Axum  
**Database:** PostgreSQL  
**Email infrastructure:** Resend API  
**Architecture:** API-driven, modular, provider-independent

---

# 1. Project Overview

You are a senior software architect, Rust backend engineer, Vue.js frontend engineer, and product designer.

Your assignment is to design and develop **KAMFANY Mail**, a modern webmail application owned by IT KAMFANY LIMITED.

KAMFANY Mail is intended to provide a centralized email management platform for managing company-owned email addresses.

The application should allow authorized users to:

- Send emails.
- Receive emails.
- Read and manage incoming messages.
- Reply to emails.
- Forward messages.
- Manage multiple email addresses.
- Organize emails into folders.
- Manage contacts.
- Manage attachments.
- Search email conversations.
- Manage email signatures.
- Configure account preferences.
- Access their mailboxes securely.

The product should offer a user experience comparable to Gmail, Outlook, and Proton Mail.

However, it must have its own distinctive identity and design.

**Product philosophy:**

Simple, elegant, secure, and productive.

# 2. Core Architecture

Build the application using a modular architecture.

The application consists of three major components.

### A. Frontend — Vue 3

Responsible for:

- User interface.
- Navigation.
- Inbox.
- Email reading.
- Email composition.
- Contacts.
- Mailbox switching.
- Settings.
- Notifications.
- User interactions.

### B. Backend — Rust + Axum

Responsible for:

- Authentication.
- Authorization.
- User management.
- Mailbox management.
- Resend API integration.
- Email sending.
- Incoming email processing.
- Message synchronization.
- Email organization.
- Attachment management.
- Webhook processing.
- Database operations.
- Background jobs.
- Real-time notifications.

### C. Email Infrastructure — Resend

Responsible for supported email-provider operations:

- Outgoing email delivery.
- Incoming email reception.
- Inbox functionality where supported.
- Message retrieval.
- Conversation operations where supported.
- Delivery events.
- Webhook notifications.

Do not assume Resend provides traditional IMAP or POP3 mailboxes.

Do not assume every feature available in Gmail is available through Resend.

Verify the current Resend API documentation before implementing provider-specific operations.

Official documentation:
- https://resend.com/docs
- https://resend.com/docs/dashboard/receiving
- https://resend.com/docs/api-reference

### Architecture principle

The frontend must communicate exclusively with the Rust backend.

The Rust backend communicates with Resend.

Never expose Resend API keys or other provider credentials to the frontend.

The backend must implement an email provider abstraction so that another email provider can be integrated in the future.

# 3. Technology Stack

## Frontend

- Vue 3
- TypeScript
- Vite
- Composition API
- Tailwind CSS
- Pinia
- Vue Router
- Axios
- Lucide Vue Next
- TipTap rich-text editor
- VueUse
- Day.js

Use `<script setup lang="ts">` throughout the application.

## Backend

- Rust
- Tokio
- Axum
- SQLx
- PostgreSQL
- Serde
- Reqwest
- Tracing
- Tower HTTP
- UUID
- Chrono
- Validator
- Argon2
- JWT or secure session-based authentication
- Redis where needed for queues, caching, and distributed coordination.

Use asynchronous Rust throughout the backend.

## Infrastructure

- Ubuntu Linux.
- Nginx reverse proxy.
- PostgreSQL.
- Redis.
- S3-compatible object storage for attachments.
- Systemd for process management.
- Resend for email infrastructure.

# 4. Application Design and Branding

Create a premium webmail interface.

The application must be visually clean, professional, and intuitive.

### Design direction

Take inspiration from modern productivity applications.

Avoid copying Gmail or Outlook directly.

Use:

- Minimalist layouts.
- Consistent spacing.
- Subtle borders.
- Soft shadows.
- Rounded corners.
- Professional typography.
- Carefully designed hover states.
- Smooth transitions.
- Responsive layouts.

### Theme

Implement:

- Light mode.
- Dark mode.
- System preference.

Use a centralized design token system for colors, spacing, typography, borders, and shadows.

### Application identity

Brand: KAMFANY Mail

Company: IT KAMFANY LIMITED

Primary URL: `mail.kamfany.com`

Create a clean, professional logo treatment and application icon.

# 5. Main Application Layout

The main interface should consist of:

1. Navigation sidebar.
2. Top navigation bar.
3. Email list.
4. Email reading panel.

## 5.1 Sidebar

### Brand section

- KAMFANY Mail logo.
- Product name.
- Mailbox/account selector.

### Compose action

A prominent Compose button.

### Default folders

- Inbox.
- Starred.
- Snoozed.
- Sent.
- Drafts.
- Archive.
- Spam.
- Trash.

### Custom folders

Users should be able to:

- Create folders.
- Rename folders.
- Delete folders.
- Assign folder colors.
- Move messages between folders.

### Labels

Support:

- Custom labels.
- Multiple labels per email.
- Label colors.
- Label filtering.

The sidebar must be collapsible.

## 5.2 Top navigation

Include:

- Sidebar toggle.
- Global search.
- Advanced search filters.
- Refresh.
- Notifications.
- Theme switcher.
- Help.
- User profile menu.

## 5.3 Email list

Each row should display:

- Selection checkbox.
- Star button.
- Sender name.
- Sender email.
- Subject.
- Message preview.
- Attachment indicator.
- Received date.
- Unread indicator.
- Important marker.

Implement:

- Individual selection.
- Select all.
- Bulk archive.
- Bulk delete.
- Bulk move.
- Bulk mark as read.
- Bulk mark as unread.
- Bulk label assignment.
- Star and unstar.
- Pagination or cursor-based loading.

Unread messages must be visually distinguishable.

## 5.4 Email reading panel

Display:

- Subject.
- Sender.
- Sender email.
- Recipient.
- CC and BCC.
- Date and time.
- Full email content.
- Attachments.

Actions:

- Reply.
- Reply all.
- Forward.
- Archive.
- Delete.
- Star.
- Mark unread.
- Move to folder.
- Add label.
- Print.

Support HTML and plain-text email content.

Render external email HTML in an isolated, sandboxed context.

Sanitize email content and block unsafe scripts, event handlers, and potentially malicious resources.

# 6. Email Composition

Create a modern email composer.

Fields:

- From.
- To.
- CC.
- BCC.
- Subject.
- Rich-text content.
- Attachments.

Use TipTap as the rich-text editor.

### Formatting

Support:

- Bold.
- Italic.
- Underline.
- Text color.
- Headings.
- Numbered lists.
- Bulleted lists.
- Hyperlinks.
- Blockquotes.
- Text alignment.

### Composer functionality

- Send email.
- Save draft.
- Autosave draft.
- Discard draft.
- Add attachments.
- Remove attachments.
- Reply.
- Reply all.
- Forward.

Support recipient autocomplete.

Allow users to select only sender addresses for which they have permission.

The backend must validate sender authorization before sending.

# 7. Multiple Mailboxes

KAMFANY Mail must support multiple company email addresses.

Examples:

- developer@kamfany.com
- admin@kamfany.com
- support@kamfany.com
- finance@kamfany.com
- info@kamfany.com

Implement a mailbox management module.

Features:

- Register mailbox identities.
- Associate supported Resend inbox identities.
- Assign users to mailboxes.
- Define mailbox permissions.
- Enable or disable mailbox access.
- Set default mailbox.
- Configure sender display names.

Do not assume that creating an email address in the application automatically creates a working receiving mailbox.

The backend must verify that the corresponding address has been configured and routed through the email provider.

Support shared mailbox access.

# 8. Contacts Management

Create a contacts module.

Contact fields:

- Full name.
- Email address.
- Phone number.
- Company.
- Profile image.
- Notes.

Features:

- Create contact.
- Edit contact.
- Delete contact.
- Search contacts.
- Import contacts.
- Autocomplete recipients.

# 9. Search

Build a powerful email search system.

Support searching by:

- Sender.
- Recipient.
- Subject.
- Message body.
- Date range.
- Attachments.
- Mailbox.
- Folder.
- Labels.

Use PostgreSQL indexing and full-text search where appropriate.

Design the search service so that it can be optimized independently.

# 10. Rust Backend Architecture

Use Axum for the HTTP API.

Organize the backend into modular services.

Suggested structure:

```text
kamfany-mail/
│
├── frontend/
│   ├── src/
│   ├── public/
│   ├── package.json
│   └── vite.config.ts
│
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   │
│   ├── migrations/
│   │
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── config.rs
│       ├── state.rs
│       │
│       ├── api/
│       │   ├── mod.rs
│       │   ├── routes.rs
│       │   ├── middleware.rs
│       │   └── errors.rs
│       │
│       ├── auth/
│       │   ├── mod.rs
│       │   ├── handlers.rs
│       │   ├── service.rs
│       │   └── models.rs
│       │
│       ├── mailboxes/
│       ├── messages/
│       ├── threads/
│       ├── folders/
│       ├── labels/
│       ├── contacts/
│       ├── attachments/
│       ├── notifications/
│       ├── webhooks/
│       │
│       ├── providers/
│       │   ├── mod.rs
│       │   ├── traits.rs
│       │   └── resend/
│       │       ├── mod.rs
│       │       ├── client.rs
│       │       ├── inboxes.rs
│       │       ├── sending.rs
│       │       ├── receiving.rs
│       │       └── webhooks.rs
│       │
│       ├── database/
│       ├── storage/
│       ├── jobs/
│       └── utils/
│
└── README.md
```

Adjust this structure as the project grows.

Do not put all business logic in `main.rs`.

Use clear module boundaries.

# 11. Email Provider Abstraction

This is a critical architectural requirement.

Do not couple the application directly to Resend.

Define an email provider interface.

Example conceptual interface:

```rust
#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send_email(
        &self,
        request: SendEmailRequest,
    ) -> Result<SendEmailResponse>;

    async fn get_message(
        &self,
        message_id: &str,
    ) -> Result<EmailMessage>;

    async fn list_inboxes(
        &self,
    ) -> Result<Vec<ProviderInbox>>;
}
```

Extend the interface only with operations supported by the provider.

For example, conversation retrieval, replies, and inbound message retrieval should be implemented only where supported by the selected Resend API product.

Create a `ResendProvider` implementation.

All provider-specific request and response structures must remain inside the provider module.

Use internal application models to prevent Resend response formats from spreading throughout the application.

The application must remain independent of the email provider.

# 12. Database Design

Use PostgreSQL and SQLx.

Create migrations for the following initial tables:

### Identity and access

- users
- roles
- permissions
- user_roles
- role_permissions
- sessions

### Mailbox management

- mailboxes
- mailbox_users
- mailbox_settings

### Email organization

- folders
- labels
- messages
- threads
- recipients
- message_labels
- message_folder_assignments
- attachments

### Other functionality

- contacts
- drafts
- notifications
- webhook_events
- audit_logs
- user_preferences
- email_signatures

### Important data requirements

Messages should store:

- Internal message ID.
- Provider message ID.
- Provider name.
- Mailbox ID.
- Thread ID.
- Sender.
- Recipients.
- Subject.
- Plain-text body.
- HTML body or a secure reference to stored content.
- Received timestamp.
- Sent timestamp.
- Read status.
- Star status.
- Attachment metadata.

Store provider identifiers separately from internal IDs.

Use UUID primary keys.

Create appropriate indexes for message searches and mailbox queries.

Avoid unnecessary duplication.

Implement database transactions for operations involving multiple related records.

# 13. Email Receiving Architecture

Implement incoming email processing through the supported Resend receiving mechanisms.

Expected flow:

```text
External Sender
      |
      v
Resend Receiving Infrastructure
      |
      v
Resend Webhook
      |
      v
Rust Webhook Endpoint
      |
      v
Verify Webhook Signature
      |
      v
Queue Email Processing Job
      |
      v
Retrieve Message and Attachments
      |
      v
Validate and Normalize
      |
      v
Save to PostgreSQL
      |
      v
Notify Authorized Mailbox Users
      |
      v
Vue Inbox Updates
```

Requirements:

- Verify webhook signatures.
- Handle duplicate events.
- Make processing idempotent.
- Preserve original provider IDs.
- Store attachments safely.
- Handle temporary provider failures.
- Implement retries with backoff.
- Log processing failures.
- Avoid blocking webhook requests during heavy processing.

Use a persistent queue rather than relying on in-memory background tasks for critical email processing.

# 14. Email Sending Architecture

Expected flow:

```text
Vue Compose Interface
       |
       v
Rust API
       |
       v
Validate User and Mailbox Permissions
       |
       v
Validate Recipients and Attachments
       |
       v
Create Outgoing Message Record
       |
       v
Resend Sending API
       |
       v
Delivery Response
       |
       v
Update Message Status
       |
       v
Notify Frontend
```

Requirements:

- Validate sender identity.
- Validate recipients.
- Enforce attachment limits.
- Support CC and BCC.
- Store message metadata.
- Handle provider errors.
- Track delivery events.
- Prevent duplicate sending where possible.

Use background jobs for appropriate asynchronous operations.

Do not mark a message as successfully delivered merely because the sending API accepted the request.

Distinguish between:

- Queued.
- Sending.
- Accepted by provider.
- Delivered.
- Bounced.
- Failed.

# 15. Attachments

Implement attachment management.

Requirements:

- Multiple attachments.
- File size validation.
- File type validation.
- Safe file names.
- Secure storage.
- Download authorization.
- Attachment previews where supported.
- Malware scanning integration.

Use S3-compatible object storage.

Avoid storing large binary attachments directly in PostgreSQL.

Do not expose permanent public attachment URLs.

# 16. API Design

Use RESTful endpoints.

Base URL:

`/api/v1`

### Authentication

```http
POST /api/v1/auth/login
POST /api/v1/auth/logout
POST /api/v1/auth/refresh
GET  /api/v1/auth/me
POST /api/v1/auth/forgot-password
POST /api/v1/auth/reset-password
```

### Mailboxes

```http
GET    /api/v1/mailboxes
GET    /api/v1/mailboxes/{id}
POST   /api/v1/mailboxes
PATCH  /api/v1/mailboxes/{id}
DELETE /api/v1/mailboxes/{id}
```

### Messages

```http
GET    /api/v1/mailboxes/{id}/messages
GET    /api/v1/messages/{id}
POST   /api/v1/messages/send
POST   /api/v1/messages/{id}/reply
POST   /api/v1/messages/{id}/forward
PATCH  /api/v1/messages/{id}/read
PATCH  /api/v1/messages/{id}/star
POST   /api/v1/messages/{id}/archive
POST   /api/v1/messages/{id}/move
DELETE /api/v1/messages/{id}
```

### Folders

```http
GET    /api/v1/folders
POST   /api/v1/folders
PATCH  /api/v1/folders/{id}
DELETE /api/v1/folders/{id}
```

### Contacts

```http
GET    /api/v1/contacts
POST   /api/v1/contacts
PATCH  /api/v1/contacts/{id}
DELETE /api/v1/contacts/{id}
```

### Search

```http
GET /api/v1/search?q=
```

### Webhooks

```http
POST /api/v1/webhooks/resend
```

These are proposed application endpoints, not Resend endpoints.

Use appropriate HTTP status codes and consistent JSON response structures.

Generate OpenAPI documentation for the backend.

# 17. Authentication and Security

Implement production-grade authentication.

Requirements:

- Secure login.
- Password hashing using Argon2.
- Session management.
- Secure token handling.
- Two-factor authentication.
- Role-based access control.
- Mailbox-level authorization.
- Rate limiting.
- Login history.
- Audit logs.
- Session revocation.

Prefer secure, HttpOnly, SameSite cookies for browser authentication where the deployment architecture permits.

Protect against:

- CSRF.
- XSS.
- SQL injection.
- IDOR.
- Brute-force attacks.
- Unauthorized mailbox access.
- Webhook replay.
- Malicious attachments.
- HTML email injection.

Never trust mailbox IDs or message IDs supplied by the frontend without checking the user's permissions.

# 18. Real-time Notifications

Implement WebSocket support through the Rust backend.

When an incoming email arrives:

- Update unread counts.
- Notify authorized users.
- Update the inbox.
- Display a notification.
- Prevent duplicate updates.

Real-time notifications must respect mailbox permissions.

Do not broadcast private email information to unauthorized users.

# 19. Vue Frontend Structure

Use the following suggested structure:

```text
frontend/src/
│
├── assets/
├── components/
│   ├── common/
│   ├── layout/
│   ├── mail/
│   ├── compose/
│   ├── contacts/
│   └── settings/
│
├── layouts/
│   ├── AuthLayout.vue
│   └── MailLayout.vue
│
├── pages/
│   ├── auth/
│   ├── mail/
│   ├── contacts/
│   └── settings/
│
├── router/
├── stores/
├── services/
├── composables/
├── types/
├── utils/
├── styles/
├── App.vue
└── main.ts
```

Create Pinia stores for:

- Authentication.
- Mailboxes.
- Messages.
- Threads.
- Folders.
- Labels.
- Contacts.
- Notifications.
- User preferences.

Keep API operations in dedicated service modules.

# 20. Frontend API Integration

Configure Axios with:

`VITE_API_BASE_URL`

Create services:

- auth.service.ts
- mailbox.service.ts
- message.service.ts
- thread.service.ts
- folder.service.ts
- label.service.ts
- contact.service.ts
- attachment.service.ts
- search.service.ts
- settings.service.ts

Implement:

- Request interceptors.
- Response interceptors.
- Authentication handling.
- Error handling.
- Request cancellation where appropriate.
- Loading indicators.

Never place Resend API credentials in frontend environment variables.

# 21. Development Phases

Build the project incrementally.

Do not generate the entire application in one step.

## Phase 1 — Project Foundation

### Frontend

- Initialize Vue 3.
- Configure Vite.
- Configure TypeScript.
- Install Tailwind CSS.
- Configure Vue Router.
- Configure Pinia.
- Install Lucide icons.
- Create global design tokens.
- Build responsive application layout.
- Build sidebar.
- Build top navigation.
- Implement light and dark themes.

### Backend

- Initialize Rust project.
- Configure Tokio.
- Configure Axum.
- Configure SQLx.
- Configure PostgreSQL.
- Configure environment variables.
- Establish application state.
- Configure structured logging.
- Create health-check endpoint.
- Configure error handling.
- Create initial database migrations.

Deliverable: A working frontend and backend foundation.

## Phase 2 — Webmail Interface

Build:

- Inbox.
- Email list.
- Email reading panel.
- Sidebar navigation.
- Folder navigation.
- Search interface.
- Message actions.
- Responsive mobile layout.

Use realistic mock data.

Keep mock data isolated so it can be replaced with API responses later.

Deliverable: A fully navigable webmail interface.

## Phase 3 — Authentication and Mailbox Management

Build:

- User authentication.
- Session handling.
- Mailbox management.
- Mailbox switching.
- User permissions.
- Account settings.

Deliverable: Users can securely access their authorized mailboxes.

## Phase 4 — Resend Integration

Before implementation, inspect the latest official Resend API documentation.

Implement:

- Resend API client.
- Provider abstraction.
- Sending API.
- Inboxes API where available and appropriate.
- Receiving API.
- Webhook handling.
- Message retrieval.
- Error handling.
- Retry mechanisms.

Test actual email sending and receiving.

Deliverable: Functional email infrastructure integration.

## Phase 5 — Message Management

Implement:

- Incoming messages.
- Outgoing messages.
- Threads.
- Reply.
- Reply all.
- Forward.
- Read/unread.
- Starred.
- Archive.
- Trash.
- Spam.
- Custom folders.
- Labels.

Deliverable: Functional email management.

## Phase 6 — Compose and Attachments

Implement:

- Rich-text editor.
- Recipient autocomplete.
- CC and BCC.
- Attachments.
- Drafts.
- Email signatures.
- Sending status.

Deliverable: Complete email composition functionality.

## Phase 7 — Contacts, Search and Notifications

Implement:

- Contact management.
- Advanced search.
- WebSocket notifications.
- Unread counters.
- Real-time inbox updates.

Deliverable: Enhanced productivity features.

## Phase 8 — Security and Production

Implement:

- Security review.
- Authorization testing.
- Rate limiting.
- Audit logs.
- Attachment scanning.
- Database backup strategy.
- Queue monitoring.
- Production build.
- Nginx configuration.
- Systemd deployment.

Deliverable: Production-ready KAMFANY Mail.

# 22. Testing Requirements

Write tests throughout development.

### Rust

- Unit tests.
- Integration tests.
- API tests.
- Database tests.
- Authentication tests.
- Permission tests.
- Provider adapter tests.
- Webhook signature tests.
- Idempotency tests.

### Vue

- Component tests.
- Store tests.
- API service tests.
- Form validation tests.
- Authentication tests.
- Email composition tests.
- Responsive interface tests.

Use mock provider responses for automated tests.

Do not use production Resend credentials in test environments.

# 23. Development Rules for the AI Agent

Follow these rules strictly.

1. Build one phase at a time.
2. Never skip architecture planning.
3. Use idiomatic Rust.
4. Use Vue 3 Composition API.
5. Use TypeScript throughout the frontend.
6. Keep business logic out of Vue components.
7. Keep provider-specific logic isolated.
8. Never expose API secrets.
9. Never hardcode production credentials.
10. Use database migrations.
11. Use transactions where appropriate.
12. Use structured logging.
13. Handle all errors properly.
14. Write tests for critical functionality.
15. Do not introduce unnecessary dependencies.
16. Do not implement fake production functionality.
17. Do not assume unsupported Resend API capabilities.
18. Maintain documentation.
19. Keep the frontend and backend independently deployable.
20. Preserve backward compatibility when changing API contracts.

# 24. AI Agent Execution Instructions

Before starting:

1. Study the project requirements.
2. Explain the proposed architecture.
3. Present the project directory structure.
4. Identify required Rust crates and npm packages.
5. Explain the database design.
6. Explain the Resend integration approach.
7. Identify external dependencies and limitations.
8. Present the implementation plan.

Then begin Phase 1.

After each phase:

- List files created.
- Explain the implemented features.
- Show relevant API endpoints.
- Run tests.
- Run frontend build.
- Run Rust checks.
- Report any errors.
- Document incomplete functionality.
- Wait for approval before starting the next phase.

Do not silently skip requirements.

If a requirement depends on a Resend API feature that is unavailable, explain the limitation and propose an alternative before implementing it.

# 25. Final Product Vision

KAMFANY Mail should become a reliable, secure, company-owned webmail platform.

The first release should serve IT KAMFANY LIMITED's internal email management requirements.

The architecture should allow future expansion into a multi-tenant email management platform for other organizations.

Potential future capabilities:

- Multi-organization support.
- Custom domains.
- Organization administrators.
- Team inboxes.
- Email aliases.
- Advanced email analytics.
- Email automation.
- AI email assistance.
- Email classification.
- Intelligent spam detection.
- Calendar integration.

Do not implement these future features prematurely.

Prioritize a stable and functional first release.

**Final instruction: Start with Phase 1. Do not attempt to build all phases at once.**
