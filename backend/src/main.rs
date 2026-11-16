use std::time::Duration;

use kamfany_mail::bootstrap;
use kamfany_mail::config::{AppConfig, LogFormat};
use kamfany_mail::database;
use kamfany_mail::delivery;
use kamfany_mail::router;
use kamfany_mail::state::build_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Serve,
    Migrate,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    init_tracing(&config);

    match command_from_args(std::env::args().skip(1))? {
        Command::Migrate => migrate(&config).await,
        Command::Serve => serve(config).await,
    }
}

fn command_from_args<I>(args: I) -> anyhow::Result<Command>
where
    I: IntoIterator<Item = String>,
{
    match args.into_iter().next().as_deref() {
        None | Some("serve") => Ok(Command::Serve),
        Some("migrate") => Ok(Command::Migrate),
        Some(other) => anyhow::bail!("unknown command '{other}'. Use serve or migrate"),
    }
}

fn init_tracing(config: &AppConfig) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=warn"));

    match config.log_format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}

async fn migrate(config: &AppConfig) -> anyhow::Result<()> {
    let url = config
        .database_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("DATABASE_URL is required to migrate"))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .map_err(|_| anyhow::anyhow!("database connection failed"))?;
    database::migrate(&pool).await?;
    tracing::info!("migrations applied");
    Ok(())
}

async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let addr = config.socket_addr()?;
    let state = build_state(config).await?;
    if let Some(pool) = &state.db {
        bootstrap::ensure_dev_admin(pool, &state.config).await?;
    }
    if state.config.resend.webhook_secret.is_none() {
        tracing::warn!("webhook signing is not set; inbound events will be refused");
    }
    if state.config.scanner.is_none() {
        tracing::warn!("no malware scanner is configured; attachments stay unscanned");
    }
    let jobs = tokio::spawn(delivery::run_jobs(state.clone()));
    let app = router(state)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    jobs.abort();
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::{Command, command_from_args};

    #[test]
    fn serve_is_the_default_command() {
        assert_eq!(
            command_from_args(Vec::<String>::new()).unwrap(),
            Command::Serve
        );
        assert_eq!(
            command_from_args(["serve".to_string()]).unwrap(),
            Command::Serve
        );
    }

    #[test]
    fn migrate_is_recognized() {
        assert_eq!(
            command_from_args(["migrate".to_string()]).unwrap(),
            Command::Migrate
        );
    }

    #[test]
    fn unknown_commands_fail() {
        assert!(command_from_args(["deploy".to_string()]).is_err());
    }
}
