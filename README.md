# KAMFANY Mail

Company webmail for IT KAMFANY LIMITED. The desk will live at `mail.kamfany.com`.

The browser talks only to the Rust API. The API will talk to Resend. Provider credentials never go to the frontend.

The master brief is [docs/KAMFANY_MAIL_IMPLEMENTATION_GUIDE.md](docs/KAMFANY_MAIL_IMPLEMENTATION_GUIDE.md). Architecture and the current phase are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/PHASE_STATUS.md](docs/PHASE_STATUS.md).

## Layout

```text
kamfany-mail/
├── frontend/     Vue 3 desk
├── backend/      Rust + Axum API
└── docs/         Brief, architecture, and phase status
```

The two apps build and deploy separately.

## Backend

Requirements: Rust, PostgreSQL.

```bash
cd backend
cp .env.example .env
createdb kamfany_mail
cargo run -- migrate
cargo run -- serve
```

The API listens on `http://127.0.0.1:8080`.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

## Frontend

Requirements: Node.js.

```bash
cd frontend
cp .env.example .env
npm install
npm run dev
```

The desk listens on `http://127.0.0.1:5173` and proxies `/api` to the Rust API.

```bash
npm test
npm run build
```

Leave `VITE_API_BASE_URL` empty during development. Set it to the public API origin only when the desk is served without the Vite proxy. Do not put Resend credentials in frontend environment variables.
