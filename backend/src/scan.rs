//! ClamAV `INSTREAM` scanning.
//!
//! When `CLAMAV_ADDRESS` is unset, a file stays `pending`. Nothing is marked
//! clean without a scanner reply.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::api::AppError;
use crate::storage::BlobStore;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn status_for(address: Option<&str>, bytes: &[u8]) -> &'static str {
    match classify(address, bytes).await {
        Verdict::Pending => "pending",
        Verdict::Clean => "clean",
        Verdict::Rejected => "rejected",
        Verdict::Failed => "failed",
    }
}

pub async fn gate_draft(
    pool: &PgPool,
    storage: &BlobStore,
    address: Option<&str>,
    draft_id: Uuid,
) -> Result<(), AppError> {
    let rejected = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM attachments WHERE draft_id = $1 AND scan_status = 'rejected'",
    )
    .bind(draft_id)
    .fetch_one(pool)
    .await?;
    if rejected > 0 {
        return Err(AppError::Validation(
            "An attachment was rejected by the scanner.".into(),
        ));
    }
    let Some(address) = address else {
        return Ok(());
    };
    let rows = sqlx::query_as::<_, PendingFile>(
        r#"
        SELECT id, storage_key
        FROM attachments
        WHERE draft_id = $1 AND scan_status IN ('pending', 'failed')
        "#,
    )
    .bind(draft_id)
    .fetch_all(pool)
    .await?;
    for row in rows {
        let bytes = storage.get(&row.storage_key).await?;
        let status = status_for(Some(address), &bytes).await;
        sqlx::query("UPDATE attachments SET scan_status = $2 WHERE id = $1")
            .bind(row.id)
            .bind(status)
            .execute(pool)
            .await?;
        if status == "rejected" {
            return Err(AppError::Validation(
                "An attachment was rejected by the scanner.".into(),
            ));
        }
        if status != "clean" {
            return Err(AppError::Validation(
                "An attachment could not be scanned. Try again when the scanner is reachable."
                    .into(),
            ));
        }
    }
    Ok(())
}

enum Verdict {
    Pending,
    Clean,
    Rejected,
    Failed,
}

async fn classify(address: Option<&str>, bytes: &[u8]) -> Verdict {
    let Some(address) = address else {
        return Verdict::Pending;
    };
    match instream(address, bytes).await {
        Ok(reply) if reply.contains("FOUND") => {
            let name = reply
                .split(':')
                .nth(1)
                .unwrap_or("malware")
                .replace("FOUND", "");
            let name = name.trim_matches(|character: char| character.is_whitespace() || character == '\0');
            tracing::warn!(signature = %name, "attachment rejected by the scanner");
            Verdict::Rejected
        }
        Ok(reply) if reply.contains("OK") => Verdict::Clean,
        Ok(reply) => {
            tracing::warn!(reply, "scanner reply was not understood");
            Verdict::Failed
        }
        Err(error) => {
            tracing::warn!(%error, "attachment scan failed");
            Verdict::Failed
        }
    }
}

async fn instream(address: &str, bytes: &[u8]) -> Result<String, std::io::Error> {
    tokio::time::timeout(Duration::from_secs(15), read_instream(address, bytes))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "scanner timed out"))?
}

async fn read_instream(address: &str, bytes: &[u8]) -> Result<String, std::io::Error> {
    let mut stream = TcpStream::connect(address).await?;
    stream.write_all(b"zINSTREAM\0").await?;
    for chunk in bytes.chunks(8192) {
        stream.write_all(&(chunk.len() as u32).to_be_bytes()).await?;
        stream.write_all(chunk).await?;
    }
    stream.write_all(&0u32.to_be_bytes()).await?;
    stream.flush().await?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).await?;
    Ok(reply)
}

#[derive(sqlx::FromRow)]
struct PendingFile {
    id: Uuid,
    storage_key: String,
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn serve(reply: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut command = [0u8; 10];
            socket.read_exact(&mut command).await.unwrap();
            assert_eq!(&command, b"zINSTREAM\0");
            loop {
                let mut len = [0u8; 4];
                socket.read_exact(&mut len).await.unwrap();
                let size = u32::from_be_bytes(len);
                if size == 0 {
                    break;
                }
                let mut chunk = vec![0u8; size as usize];
                socket.read_exact(&mut chunk).await.unwrap();
            }
            socket.write_all(reply).await.unwrap();
            socket.shutdown().await.unwrap();
        });
        address
    }

    #[tokio::test]
    async fn a_clean_reply_marks_the_file_clean() {
        let address = serve(b"stream: OK\0").await;
        assert_eq!(super::status_for(Some(&address), b"hello desk").await, "clean");
    }

    #[tokio::test]
    async fn a_found_reply_rejects_the_file() {
        let address = serve(b"stream: Eicar-Test-Signature FOUND\0").await;
        assert_eq!(super::status_for(Some(&address), b"EICAR").await, "rejected");
    }

    #[tokio::test]
    async fn an_unconfigured_scanner_leaves_the_file_pending() {
        assert_eq!(super::status_for(None, b"hello").await, "pending");
    }
}
