//! ViewOS face tracking daemon.
//!
//! Captures webcam frames, locates the viewer's face with a locally executed
//! YuNet model, estimates where the head is relative to the screen, and
//! publishes that position on a Unix socket for the KWin effect to consume.
//!
//! Nothing leaves the machine. The crate has no networking code at all beyond
//! the `AF_UNIX` listener, which is what `PRIVACY.md` claims; the systemd unit
//! enforces the same restriction independently with `RestrictAddressFamilies`
//! and `PrivateNetwork`.
//!
//! The library half exists so the modules can be unit tested without
//! constructing a camera.

pub mod config;
pub mod detector;
pub mod pose;
pub mod socket;
pub mod types;

pub use config::DaemonConfig;
pub use types::{FaceDetection, HeadPose, HeadPosition, PROTOCOL_VERSION};

/// Default socket path for the current session.
pub use socket::default_socket_path;
