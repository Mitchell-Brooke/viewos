//! ViewOS Face Tracking Daemon
//!
//! A Rust daemon that captures webcam frames, detects faces using YuNet,
//! solves head pose, and publishes tilt angles for the KWin effect.

mod config;
mod detector;
mod pose;
mod socket;
mod types;

pub use config::DaemonConfig;
pub use types::{FaceTiltData, HeadPose, FaceDetection};