//! Authenticated client for requesting console snapshots.

use std::{io, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
    time::timeout,
};

use super::{
    protocol::{
        AUTH_CHALLENGE_LEN, AUTH_DENIED, AUTH_OK, AUTH_TAG_LEN, MAX_FRAME_BYTES, parse_challenge,
        tag,
    },
    wire::{Message, Snapshot},
};

/// A TCP client for requesting snapshots from a Kameo console server.
#[derive(Debug)]
pub struct Client {
    stream: TcpStream,
    timeout: Duration,
}

impl Client {
    /// Connects to a console server and optionally authenticates with a shared token.
    pub async fn connect(
        addr: impl ToSocketAddrs,
        timeout_duration: Duration,
        auth_token: Option<&[u8]>,
    ) -> io::Result<Self> {
        let stream = timeout(timeout_duration, TcpStream::connect(addr))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "console connect timed out"))??;
        let mut client = Client {
            stream,
            timeout: timeout_duration,
        };
        if let Some(token) = auth_token {
            client.authenticate(token).await?;
        }
        Ok(client)
    }

    /// Requests the latest actor-system snapshot.
    pub async fn snapshot(&mut self) -> io::Result<Snapshot> {
        timeout(self.timeout, async {
            self.stream.write_all(&[0]).await?;

            let mut len = [0u8; 4];
            self.stream.read_exact(&mut len).await?;
            let len = u32::from_be_bytes(len);
            if len > MAX_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("snapshot frame too large ({len} bytes)"),
                ));
            }

            let mut bytes = vec![0; len as usize];
            self.stream.read_exact(&mut bytes).await?;
            let Message::Snapshot(snapshot) = rmp_serde::from_slice(&bytes)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            Ok(snapshot)
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "console operation timed out"))?
    }

    async fn authenticate(&mut self, token: &[u8]) -> io::Result<()> {
        timeout(self.timeout, async {
            let mut challenge = [0; AUTH_CHALLENGE_LEN];
            self.stream.read_exact(&mut challenge).await?;
            let nonce = parse_challenge(&challenge)?;
            let response: [u8; AUTH_TAG_LEN] = tag(token, &nonce);
            self.stream.write_all(&response).await?;

            let mut status = [0u8; 1];
            self.stream.read_exact(&mut status).await?;
            match status[0] {
                AUTH_OK => Ok(()),
                AUTH_DENIED => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "console authentication failed",
                )),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid console authentication response",
                )),
            }
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "console operation timed out"))?
    }
}
