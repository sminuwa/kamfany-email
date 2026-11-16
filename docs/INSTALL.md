# Installation

Local development stays in the [README](../README.md). This guide is for a VPS that already runs another Rust application and uses aaPanel. The desk and that application stay separate: different port, database, and website directory.

Nothing here changes DNS or MX. `mail.kamfany.com` is the public site. Inbound mail still needs a later decision about where MX points.

## What runs where

The Vue desk is a set of static files. Nginx in aaPanel serves them. The Rust API is one process on localhost. The reverse proxy sends only `/api` to that process.

| Piece | Where it lives |
| --- | --- |
| Desk files | The aaPanel site directory, for example `/www/wwwroot/mail.kamfany.com` |
| API | `127.0.0.1` on a port the other Rust app is not using, for example `8090` |
| Settings | `/etc/kamfany-mail.env` |
| Attachments | A directory outside the website root, for example `/var/lib/kamfany-mail/storage` |
| Database | A PostgreSQL database used only by this desk |

`deploy/nginx/mail.kamfany.com.conf` and `deploy/systemd/kamfany-mail.service` are the same shape. On aaPanel, use the panel's site and Supervisor instead of replacing its Nginx file.

## 1. Port

The API defaults to `127.0.0.1:8080`. If the existing Rust application already listens there, choose another port. `8090` is used below. The API stays on localhost. aaPanel is the public listener.

## 2. Database

In aaPanel, create a PostgreSQL database and user for this desk only, for example database `kamfany_mail` and user `kamfany_mail`. Leave the other application's database as it is.

## 3. Build on the VPS

A binary built on a Mac does not run on the server. Install Rust and Node on the VPS, then:

```bash
git clone https://github.com/sminuwa/kamfany-email.git /opt/kamfany-mail
cd /opt/kamfany-mail/backend
cargo build --release
cargo run --release -- migrate
cd ../frontend
npm install
npm run build
```

Leave `VITE_API_BASE_URL` empty in `frontend/.env`. The browser calls `/api` on the same site, and Nginx forwards that to the API. Resend credentials stay in `/etc/kamfany-mail.env` only.

Copy the built desk into the site directory aaPanel created:

```bash
rm -rf /www/wwwroot/mail.kamfany.com/*
cp -a /opt/kamfany-mail/frontend/dist/. /www/wwwroot/mail.kamfany.com/
```

Change the path if aaPanel used a different site directory.

## 4. Environment

Create `/etc/kamfany-mail.env` from `backend/.env.example`:

```text
APP_ENV=production
APP_HOST=127.0.0.1
APP_PORT=8090
DATABASE_URL=postgres://kamfany_mail:THE_PASSWORD@127.0.0.1:5432/kamfany_mail
CORS_ORIGINS=https://mail.kamfany.com
LOG_FORMAT=json
STORAGE_DIR=/var/lib/kamfany-mail/storage
RESEND_API_KEY=
RESEND_WEBHOOK_SECRET=
RESEND_API_BASE=https://api.resend.com
RESEND_SENDING_DOMAIN=kamfany.com
CLAMAV_ADDRESS=
```

`CORS_ORIGINS` is the exact public address, including `https://`. Production session cookies are `Secure`, so the site needs the certificate from the aaPanel SSL tab. A plain HTTP site will not keep the sign-in.

Create the attachment directory and keep it outside the website root:

```bash
mkdir -p /var/lib/kamfany-mail/storage
```

`APP_ENV=production` requires `DATABASE_URL`. It does not create the first account or the company mailboxes. For the first boot only, set `APP_ENV=development` and add:

```text
BOOTSTRAP_ADMIN_EMAIL=adaeze@kamfany.com
BOOTSTRAP_ADMIN_PASSWORD=a-password-of-at-least-12-characters
```

Start the API once, sign in, then stop it. Set `APP_ENV=production` and remove `BOOTSTRAP_ADMIN_PASSWORD` from the file. A later start does not change that password.

Leave `RESEND_WEBHOOK_SECRET` and `CLAMAV_ADDRESS` empty until those services exist. Empty webhook signing refuses inbound events. An empty scanner leaves attachments marked not scanned, and sending still works.

## 5. Process

aaPanel Supervisor, or systemd, can run the binary. The working directory is `/opt/kamfany-mail/backend`.

```text
/opt/kamfany-mail/backend/target/release/kamfany-mail serve
```

Load `/etc/kamfany-mail.env` into that process. `deploy/systemd/kamfany-mail.service` is the systemd form. Change the port there if you are not using the file's default of `8080`; the environment file is what the process reads.

After it starts:

```bash
curl http://127.0.0.1:8090/api/v1/health
```

That request goes straight to the API and does not pass through Nginx.

## 6. Reverse proxy

The proxy you already added should target `http://127.0.0.1:8090` and apply only to the directory `/api`. The site root remains the Vue `index.html`. Sending the whole domain to the API returns JSON for every page.

In that proxy's configuration:

```nginx
proxy_http_version 1.1;
proxy_set_header Host $host;
proxy_set_header X-Forwarded-For $remote_addr;
proxy_set_header X-Forwarded-Proto $scheme;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection $connection_upgrade;
proxy_read_timeout 3600s;
client_max_body_size 12m;
```

Use `$remote_addr` for `X-Forwarded-For`. Sign-in rate limits take the first address in that header. `$proxy_add_x_forwarded_for` would trust an address supplied by the browser.

Enable WebSocket on this proxy. The desk opens `wss://mail.kamfany.com/api/v1/ws`, and the long read timeout keeps that socket open.

In the site configuration, Vue's history routes need:

```nginx
location / {
    try_files $uri $uri/ /index.html;
}
```

Issue the certificate for `mail.kamfany.com` from the aaPanel SSL tab, then reload Nginx.

## 7. Backups

`deploy/backup-database.sh` runs `pg_dump` using `DATABASE_URL` and keeps 14 days of files in `BACKUP_DIR` (default `/var/backups/kamfany-mail`). `deploy/systemd/kamfany-mail-backup.service` and `kamfany-mail-backup.timer` run it daily. Copying those dumps off the server is a separate step.

## 8. After the site answers

These addresses should respond:

- `https://mail.kamfany.com` is the desk.
- `https://mail.kamfany.com/api/v1/health` is the API.
- A later Resend webhook URL is `https://mail.kamfany.com/api/v1/webhooks/resend`.

Receiving mail still needs `RESEND_WEBHOOK_SECRET`, receiving enabled on a domain, and that domain's MX pointed at Resend. Leave the `kamfany.com` MX records unchanged until that is an explicit decision.

A file stays `pending` until `CLAMAV_ADDRESS` points at clamd. A rejected file is not sent. If the scanner fails while receiving mail, the message is stored and the file is marked `failed`.
