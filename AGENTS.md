# KAMFANY Mail

Read `docs/KAMFANY_MAIL_IMPLEMENTATION_GUIDE.md` before making changes. It is the master brief for this repository.

Build one phase at a time. The active phase and its gaps are recorded in `docs/PHASE_STATUS.md`. Do not start the next phase until the current one is approved.

The frontend talks only to the Rust API. Resend credentials stay on the backend, inside `backend/src/providers`, and never in frontend environment variables.
