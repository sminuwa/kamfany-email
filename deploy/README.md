# Production units

These files are the deploy shape for `mail.kamfany.com`. They are not installed by the app, and they do not change DNS or MX. The aaPanel steps are in [docs/INSTALL.md](../docs/INSTALL.md). On that panel, keep the API on a localhost port the other Rust application is not using.

`deploy/deploy-kamfany-mail` is the server update command. Install it as `/usr/local/bin/deploy-kamfany-mail`, then run `sudo deploy-kamfany-mail`. It fast-forwards `main`, rebuilds the release binary, applies pending migrations, and restarts the Supervisor program.

## API

1. Build `backend/target/release/kamfany-mail` and `frontend/dist`.
2. Copy the release binary, `frontend/dist`, `deploy/backup-database.sh`, and the migrations with the binary's working directory.
3. Put runtime settings in `/etc/kamfany-mail.env`. Use `backend/.env.example` as the list. `APP_ENV=production` requires `DATABASE_URL`. Leave `CLAMAV_ADDRESS` blank until clamd is listening. Leave `RESEND_WEBHOOK_SECRET` blank until the public webhook URL exists.
4. Install `deploy/systemd/kamfany-mail.service` and start it. The process listens on `127.0.0.1:8080`.
5. Install `deploy/nginx/mail.kamfany.com.conf`, then obtain the certificate with certbot. Nginx overwrites `X-Forwarded-For` with the connecting address.
6. Install the backup service and timer. The dump stays on the server for 14 days. Copying it off the server is a separate step.

Webhook receiving still needs a domain whose MX points at Resend, receiving enabled for that domain, and `RESEND_WEBHOOK_SECRET` set to the signing secret. Do not point `kamfany.com` at Resend until that is an explicit decision.

A missing scanner leaves attachments `pending`. A configured scanner that rejects a file blocks sending that draft. A scanner failure on inbound mail stores the message and marks the file `failed`.
