//! Newline-delimited protocol framing over a Unix socket.

use anyhow::{Context, Result};
use bh_core::Envelope;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

/// Reads one envelope, or `None` at end of stream.
pub async fn read_envelope(reader: &mut BufReader<OwnedReadHalf>) -> Result<Option<Envelope>> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .await
        .context("reading protocol frame")?;
    if read == 0 {
        return Ok(None);
    }
    Envelope::decode(&line)
        .map(Some)
        .map_err(|error| anyhow::anyhow!(error))
}

/// Writes one envelope as a newline-terminated line.
pub async fn write_envelope(writer: &mut OwnedWriteHalf, envelope: &Envelope) -> Result<()> {
    let line = envelope.encode().map_err(|error| anyhow::anyhow!(error))?;
    writer
        .write_all(line.as_bytes())
        .await
        .context("writing protocol frame")?;
    writer.flush().await.context("flushing protocol frame")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bh_core::{ProtocolMessage, RepoId};

    #[tokio::test]
    async fn round_trips_an_envelope_over_a_socket_pair() {
        let (client, server) = tokio::net::UnixStream::pair().unwrap();
        let (_, mut client_write) = client.into_split();
        let (server_read, _) = server.into_split();

        let sent = Envelope::new(ProtocolMessage::Hello {
            repo: RepoId::from("bulkhead"),
            pid: 1,
        });
        write_envelope(&mut client_write, &sent).await.unwrap();

        let mut reader = BufReader::new(server_read);
        let received = read_envelope(&mut reader).await.unwrap().unwrap();
        assert_eq!(sent, received);
    }
}
