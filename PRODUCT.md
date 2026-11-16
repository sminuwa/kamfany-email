# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Vue 3 + TypeScript + Vite frontend. Rust + Axum + SQLx backend. PostgreSQL. Resend for email delivery and receiving. Redis, S3-compatible storage, Nginx, and systemd belong to the target deployment and are introduced in the phase that needs them.

## Users

Authorized staff of IT KAMFANY LIMITED, working at a desk in a browser, sending and reading company-owned mailboxes. Shared mailbox access is part of the product. The first release is internal. Multi-organization use is a later expansion, not part of this build.

## Product Purpose

KAMFANY Mail is the company mailbox at mail.kamfany.com. It exists so IT KAMFANY LIMITED can manage its own addresses in one place: send, receive, reply, forward, file, and search. Success for the first release is a stable desk for those internal addresses.

## Positioning

The company owns the desk and the mail records. The browser never talks to the email provider. Organization such as folders, labels, and read state lives in our database, because the provider is not a traditional mailbox.

## Operating Context

People use it in a browser during the workday and after hours, so light, dark, and system themes are required. Several company addresses can be assigned to one person, including shared mailboxes. Receiving works only for addresses that have actually been routed through the provider.

## Capabilities and Constraints

Confirmed product scope, built across phases: send, receive, read, reply, forward, multiple addresses, folders, labels, contacts, attachments, search, signatures, preferences, and secure access.

Constraints that bind the build:

- Do not assume IMAP, POP3, or a Gmail-equivalent API from Resend.
- Do not treat a mailbox row as a working inbox until the provider route is verified.
- Do not mark mail delivered because the sending API accepted it.
- Provider credentials stay on the server.
- HttpOnly, SameSite cookies are the preferred browser session when the deployment allows it.
- Future items such as multi-organization support, custom domains, aliases, analytics, automation, and calendar stay out of the first release.

## Brand Commitments

Name: KAMFANY Mail. Company: IT KAMFANY LIMITED. Address: mail.kamfany.com. Voice: simple, elegant, secure, and productive. The interface must be its own, not a copy of Gmail or Outlook. Binding visual constraints from the brief: minimalist layout, consistent spacing, subtle borders, soft shadows, rounded corners, professional type, hover and motion with restraint, responsive layout, light and dark and system themes, and one token system for color, space, type, borders, and shadow.

## Evidence on Hand

No logo file, production mailbox, customer list, or Resend credential was provided. Do not invent sent mail, customer names, or a working provider account.

## Product Principles

- The provider is replaceable. The desk is ours.
- A person only sees mailboxes they have been granted.
- Accepted by the provider is not delivered.
- Build one phase at a time, and do not fake a capability that is not connected.
- Secrets and provider payloads stay on the server.
