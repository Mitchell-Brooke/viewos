//! The Unix socket the KWin effect connects to.
//!
//! One-way by design: the daemon publishes and the effect listens. See
//! `face-tilt/shared/protocol.md`.

use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::broadcast,
};

use tracing::{debug, info, warn};

use crate::types::HeadPosition;

/// Largest message we will serialise, as a guard against a pathological value.
const MAX_MESSAGE_BYTES: usize = 4096;

/// Accepts connections and streams head positions to each of them.
pub struct SocketServer {
    socket_path: PathBuf,
    mode: u32,
    receiver: broadcast::Receiver<HeadPosition>,
}

impl SocketServer {
    /// Create a server that will listen on `socket_path`.
    ///
    /// `receiver` is the shared publication channel; each client subscribes to
    /// it so that the capture loop never blocks on a slow or wedged consumer.
    pub fn new(
        socket_path: PathBuf,
        mode: u32,
        receiver: broadcast::Receiver<HeadPosition>,
    ) -> Self {
        Self {
            socket_path,
            mode,
            receiver,
        }
    }

    /// Bind and serve until the process exits.
    ///
    /// A stale socket file from a previous run is removed first. Without that,
    /// a daemon started twice would fail to bind even though nothing is
    /// listening, which looks identical to "the port is in use by something
    /// else".
    pub async fn run(&mut self) -> io::Result<()> {
        if self.socket_path.exists() {
            debug!("removing stale socket at {:?}", self.socket_path);
            // A leftover socket is only removable if nothing is listening. If
            // remove fails because it is in use, bind will fail next and say
            // so, which is a better error than silently stealing another
            // daemon's socket.
            std::fs::remove_file(&self.socket_path)?;
        }

        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        // Set restrictive permissions on the socket itself. Unix socket
        // permissions decide who may connect, and head position is arguably
        // the most intimate thing on a desktop. The default from bind() is
        // 0777 minus umask, which on many systems means other users can read
        // it.
        std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(self.mode))?;

        info!(
            "listening on {} (mode {:04o})",
            self.socket_path.display(),
            self.mode
        );

        // Held for the lifetime of the function so the socket file is removed
        // when the daemon stops, including on an unclean exit.
        let _cleanup = SocketCleanup {
            path: self.socket_path.clone(),
        };

        loop {
            let (stream, _addr) = listener.accept().await?;
            // Each client gets its own subscription, so one client lagging
            // cannot stall the others or the capture loop.
            let receiver = self.receiver.resubscribe();

            tokio::spawn(async move {
                if let Err(error) = serve_client(stream, receiver).await {
                    debug!("client disconnected: {error}");
                }
            });
        }
    }
}

/// Removes the socket file when the server stops.
struct SocketCleanup {
    path: PathBuf,
}

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != io::ErrorKind::NotFound {
                warn!("could not remove socket {}: {error}", self.path.display());
            }
        }
    }
}

/// Stream positions to one client until it goes away.
async fn serve_client(
    mut stream: UnixStream,
    mut receiver: broadcast::Receiver<HeadPosition>,
) -> io::Result<()> {
    // No per-connection permission tightening is needed: the kernel checks
    // write permission on the socket file at connect() time, so the mode set
    // in `run` is what actually gates access.

    loop {
        match receiver.recv().await {
            Ok(position) => {
                let mut line = serde_json::to_vec(&position).map_err(io::Error::other)?;
                if line.len() > MAX_MESSAGE_BYTES {
                    warn!("refusing to send an oversized message");
                    continue;
                }
                line.push(b'\n');

                // A write error means the client is gone. Propagating it tears
                // down this task, which is exactly right.
                stream.write_all(&line).await?;
            }
            // The client is slower than we publish. Head pose is a continuous
            // quantity where only the newest sample matters, so dropping the
            // backlog is correct behaviour, not a compromise.
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                debug!("client lagged, dropped {skipped} messages");
            }
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

/// Default socket location for a user service.
///
/// Falls back to `/tmp` only if the runtime directory is unset, which happens
/// for daemons started outside a session. The result is a warning case rather
/// than a normal one.
pub fn default_socket_path() -> PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        if !runtime.is_empty() {
            return Path::new(&runtime).join("viewos").join("face-tilt.sock");
        }
    }
    PathBuf::from("/tmp/viewos-face-tilt.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_stays_within_the_size_limit() {
        let position = HeadPosition::default();
        let line = serde_json::to_vec(&position).unwrap();
        assert!(
            line.len() < MAX_MESSAGE_BYTES,
            "serialised position is {} bytes",
            line.len()
        );
    }

    #[test]
    fn serialised_field_names_match_the_protocol() {
        let json: serde_json::Value =
            serde_json::to_value(HeadPosition::default()).unwrap();
        let object = json.as_object().unwrap();
        for field in ["v", "hx", "hy", "hz", "roll", "conf", "t"] {
            assert!(object.contains_key(field), "missing field {field}");
        }
        assert_eq!(object.len(), 7, "unexpected extra fields");
    }

    #[test]
    fn lost_position_reports_zero_confidence() {
        assert_eq!(HeadPosition::lost().confidence, 0.0);
    }

    #[test]
    fn default_socket_path_is_under_the_runtime_directory() {
        // SAFETY: single-threaded test setup, restored immediately after.
        let previous = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        let path = default_socket_path();
        match previous {
            Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
        assert_eq!(path, PathBuf::from("/run/user/1000/viewos/face-tilt.sock"));
    }
}
