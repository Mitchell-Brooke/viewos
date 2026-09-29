use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::types::FaceTiltData;

pub struct SocketServer {
    socket_path: std::path::PathBuf,
    rx: broadcast::Receiver<FaceTiltData>,
}

impl SocketServer {
    pub fn new(socket_path: std::path::PathBuf, rx: broadcast::Receiver<FaceTiltData>) -> Self {
        Self { socket_path, rx }
    }

    pub async fn run(mut self) -> std::io::Result<()> {
        // Remove old socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Create parent directory
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Socket server listening on {:?}", self.socket_path);

        // Set permissions so user can connect
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.socket_path)?.permissions();
            perms.set_mode(0o666);
            std::fs::set_permissions(&self.socket_path, perms)?;
        }

        loop {
            let (stream, _) = listener.accept().await?;
            let mut rx = self.rx.resubscribe();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(stream, &mut rx).await {
                    debug!("Client disconnected: {}", e);
                }
            });
        }
    }

    async fn handle_client(
        mut stream: UnixStream,
        rx: &mut broadcast::Receiver<FaceTiltData>,
    ) -> std::io::Result<()> {
        let mut buf = vec![0u8; 1024];

        loop {
            tokio::select! {
                // Send tilt data to client
                result = rx.recv() => {
                    match result {
                        Ok(data) => {
                            let json = serde_json::to_vec(&data)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                            let msg = format!("{}\n", String::from_utf8_lossy(&json));
                            stream.write_all(msg.as_bytes()).await?;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Client lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Broadcast channel closed");
                            break;
                        }
                    }
                }

                // Handle client commands (for future extensibility)
                n = stream.read(&mut buf) => {
                    match n {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                            match cmd.as_str() {
                                "ping" => {
                                    stream.write_all(b"pong\n").await?;
                                }
                                "get" => {
                                    // Send latest immediately - already handled by broadcast
                                }
                                _ => {
                                    stream.write_all(b"unknown command\n").await?;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Read error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}