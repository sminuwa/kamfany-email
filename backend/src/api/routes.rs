use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::audit::list_audit;
use crate::api::auth::{
    change_password, forgot_password, login, logout, me, refresh, reset_password, revoke_session,
    sessions, update_me,
};
use crate::api::browser::require_browser;
use crate::api::compose::{
    add_draft, add_file, add_signature, discard_draft, discard_file, discard_signature,
    mailbox_signatures, open_draft, open_file, recipient_suggestions, save_draft, save_signature,
};
use crate::api::contacts::{
    add_avatar, add_contact, avatar, contacts, discard_avatar, discard_contact, import,
    save_contact,
};
use crate::api::errors::AppError;
use crate::api::health::{health, ready};
use crate::api::live::{notices, read_notice, read_notices, socket};
use crate::api::mailboxes::{
    create_mailbox, delete_mailbox, get_mailbox, grant_member, list_mailboxes, list_members,
    make_default, revoke_member, update_mailbox,
};
use crate::api::messages::{
    add_folder, add_label_def, archive_message, bulk_messages, folders, forward, label_message,
    labels, list_mailbox_messages, move_message, patch_folder, patch_label, read_message,
    remove_folder, remove_label_def, remove_message, reply, set_read, set_star, unlabel_message,
};
use crate::api::middleware::{cors_layer, security_headers};
use crate::api::provider::{provider_status, provider_sync, resend_webhook, send_message};
use crate::api::search::search;
use crate::api::users::{create_user, list_users};
use crate::config::ConfigError;
use crate::state::AppState;

pub fn router(state: AppState) -> Result<Router, ConfigError> {
    let cors = cors_layer(&state.config.cors_origins)?;
    let browser = browser_router().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        require_browser,
    ));
    let http = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ready", get(ready))
        .route("/api/v1/webhooks/resend", post(resend_webhook))
        .merge(browser)
        .layer(DefaultBodyLimit::max(11 * 1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(60),
        ));
    Ok(Router::new()
        .route("/api/v1/ws", get(socket))
        .merge(http)
        .fallback(not_found)
        .layer(axum::middleware::from_fn(security_headers))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

fn browser_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/me", get(me).patch(update_me))
        .route("/api/v1/auth/password", post(change_password))
        .route("/api/v1/auth/sessions", get(sessions))
        .route("/api/v1/auth/sessions/{id}", delete(revoke_session))
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route(
            "/api/v1/mailboxes",
            get(list_mailboxes).post(create_mailbox),
        )
        .route(
            "/api/v1/mailboxes/{id}",
            get(get_mailbox)
                .patch(update_mailbox)
                .delete(delete_mailbox),
        )
        .route(
            "/api/v1/mailboxes/{id}/members",
            get(list_members).post(grant_member),
        )
        .route(
            "/api/v1/mailboxes/{id}/members/{user_id}",
            delete(revoke_member),
        )
        .route("/api/v1/mailboxes/{id}/default", post(make_default))
        .route("/api/v1/mailboxes/{id}/send", post(send_message))
        .route("/api/v1/mailboxes/{id}/drafts", post(add_draft))
        .route("/api/v1/mailboxes/{id}/signatures", get(mailbox_signatures))
        .route(
            "/api/v1/mailboxes/{id}/recipients",
            get(recipient_suggestions),
        )
        .route("/api/v1/signatures", post(add_signature))
        .route(
            "/api/v1/signatures/{id}",
            patch(save_signature).delete(discard_signature),
        )
        .route(
            "/api/v1/drafts/{id}",
            get(open_draft).patch(save_draft).delete(discard_draft),
        )
        .route(
            "/api/v1/drafts/{id}/attachments",
            post(add_file).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        .route(
            "/api/v1/drafts/{id}/attachments/{file_id}",
            delete(discard_file),
        )
        .route("/api/v1/attachments/{id}", get(open_file))
        .route(
            "/api/v1/mailboxes/{id}/messages",
            get(list_mailbox_messages).post(bulk_messages),
        )
        .route(
            "/api/v1/mailboxes/{id}/folders",
            get(folders).post(add_folder),
        )
        .route(
            "/api/v1/mailboxes/{id}/labels",
            get(labels).post(add_label_def),
        )
        .route(
            "/api/v1/folders/{id}",
            patch(patch_folder).delete(remove_folder),
        )
        .route(
            "/api/v1/labels/{id}",
            patch(patch_label).delete(remove_label_def),
        )
        .route(
            "/api/v1/messages/{id}",
            get(read_message).delete(remove_message),
        )
        .route("/api/v1/messages/{id}/read", patch(set_read))
        .route("/api/v1/messages/{id}/star", patch(set_star))
        .route("/api/v1/messages/{id}/archive", post(archive_message))
        .route("/api/v1/messages/{id}/move", post(move_message))
        .route("/api/v1/messages/{id}/reply", post(reply))
        .route("/api/v1/messages/{id}/forward", post(forward))
        .route("/api/v1/messages/{id}/labels", post(label_message))
        .route(
            "/api/v1/messages/{id}/labels/{label_id}",
            delete(unlabel_message),
        )
        .route("/api/v1/provider", get(provider_status).post(provider_sync))
        .route("/api/v1/audit", get(list_audit))
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/contacts", get(contacts).post(add_contact))
        .route("/api/v1/contacts/import", post(import))
        .route(
            "/api/v1/contacts/{id}",
            patch(save_contact).delete(discard_contact),
        )
        .route(
            "/api/v1/contacts/{id}/avatar",
            get(avatar).post(add_avatar).delete(discard_avatar),
        )
        .route("/api/v1/search", get(search))
        .route("/api/v1/notifications", get(notices))
        .route("/api/v1/notifications/read", post(read_notices))
        .route("/api/v1/notifications/{id}/read", post(read_notice))
}

async fn not_found() -> AppError {
    AppError::NotFound
}
