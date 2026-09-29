//! ViewOS Face Tracking Daemon
//!
//! Captures webcam frames, runs YuNet face detection, solves head pose via PnP,
//! and publishes tilt angles over a Unix socket for the KWin effect to consume.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, File};
use nalgebra::{Point3, UnitQuaternion, Vector3};
use opencv::{core, highgui, imgproc, objdetect, prelude::*, videoio};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod config;
mod detector;
mod pose;
mod socket;
mod types;

use crate::config::DaemonConfig;
use crate::detector::YuNetDetector;
use crate::pose::HeadPoseSolver;
use crate::socket::SocketServer;
use crate::types::{FaceTiltData, HeadPose};

#[derive(Parser, Debug)]
#[command(name = "viewos-face-daemon", version, about = "ViewOS face tracking daemon")]
struct Args {
    /// Config file path
    #[arg(short, long, default_value = "/etc/viewos/face-daemon.toml")]
    config: PathBuf,

    /// Socket path
    #[arg(short, long, default_value = "/run/viewos/face-tilt.sock")]
    socket: PathBuf,

    /// Camera device index
    #[arg(short, long, default_value_t = 0)]
    camera: usize,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&args.log_level))
        .unwrap();
    FmtSubscriber::builder()
        .with_env_filter(filter)
        .init();

    info!("Starting ViewOS face tracking daemon v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = load_config(&args.config).await?;
    info!("Loaded configuration from {:?}", args.config);

    // Ensure socket directory exists
    if let Some(parent) = args.socket.parent() {
        std::fs::create_dir_all(parent).context("Failed to create socket directory")?;
    }

    // Initialize camera
    let mut camera = videoio::VideoCapture::new(args.camera as i32, videoio::CAP_ANY)
        .context("Failed to open camera")?;
    camera.set(videoio::CAP_PROP_FRAME_WIDTH, 640).ok();
    camera.set(videoio::CAP_PROP_FRAME_HEIGHT, 480).ok();
    camera.set(videoio::CAP_PROP_FPS, 30).ok();
    info!("Camera opened: {}x{} @ {}fps",
        camera.get(videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(0.0) as i32,
        camera.get(videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(0.0) as i32,
        camera.get(videoio::CAP_PROP_FPS).unwrap_or(0.0) as i32);

    // Initialize detector
    let mut detector = YuNetDetector::new(&config.detector)
        .context("Failed to initialize YuNet detector")?;

    // Initialize pose solver
    let pose_solver = HeadPoseSolver::new(&config.pose)
        .context("Failed to initialize pose solver")?;

    // Start socket server
    let (tx, _rx) = broadcast::channel::<FaceTiltData>(16);
    let socket_server = SocketServer::new(args.socket.clone(), tx.subscribe());
    let socket_handle = tokio::spawn(async move {
        if let Err(e) = socket_server.run().await {
            error!("Socket server error: {}", e);
        }
    });

    // Main processing loop
    let mut frame = Mat::default();
    let mut last_publish = std::time::Instant::now();
    let publish_interval = Duration::from_millis(1000 / config.daemon.max_fps as u64);

    info!("Starting face tracking loop");

    loop {
        // Check for shutdown signal
        if signal::ctrl_c().await.is_ok() {
            info!("Shutdown signal received");
            break;
        }

        // Capture frame
        if !camera.read(&mut frame)? {
            warn!("Failed to read frame");
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        if frame.empty() {
            continue;
        }

        // Detect faces
        let faces = detector.detect(&frame)?;

        if let Some(face) = faces.first() {
            // Solve head pose
            if let Ok(pose) = pose_solver.solve(face, &frame) {
                // Convert to tilt angles
                let tilt_data = pose_to_tilt(&pose, &config.tilt);

                // Publish at configured rate
                if last_publish.elapsed() >= publish_interval {
                    if tx.send(tilt_data).is_err() {
                        debug!("No socket subscribers");
                    }
                    last_publish = std::time::Instant::now();
                }
            }
        } else {
            // No face detected - publish neutral
            if last_publish.elapsed() >= publish_interval {
                let neutral = FaceTiltData {
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    confidence: 0.0,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };
                let _ = tx.send(neutral);
                last_publish = std::time::Instant::now();
            }
        }

        // Yield to other tasks
        tokio::task::yield_now().await;
    }

    info!("Shutting down");
    socket_handle.abort();
    Ok(())
}

async fn load_config(path: &PathBuf) -> Result<DaemonConfig> {
    let mut builder = Config::builder();
    if path.exists() {
        builder = builder.add_source(File::from(path.clone()));
    } else {
        warn!("Config file not found at {:?}, using defaults", path);
    }
    builder.build()?.try_deserialize().context("Failed to parse config")
}

fn pose_to_tilt(pose: &HeadPose, tilt_config: &config::TiltConfig) -> FaceTiltData {
    // Convert head pose (yaw, pitch, roll) to window tilt angles
    // Yaw -> rotation around Y axis (left/right tilt)
    // Pitch -> rotation around X axis (up/down tilt)
    // Roll -> generally ignored or minimal

    let yaw_deg = pose.yaw.to_degrees();
    let pitch_deg = pose.pitch.to_degrees();

    // Apply deadzone
    let yaw = if yaw_deg.abs() < tilt_config.deadzone_deg {
        0.0
    } else {
        (yaw_deg.signum() * (yaw_deg.abs() - tilt_config.deadzone_deg))
            .clamp(-tilt_config.max_angle_deg, tilt_config.max_angle_deg)
    };

    let pitch = if pitch_deg.abs() < tilt_config.deadzone_deg {
        0.0
    } else {
        (pitch_deg.signum() * (pitch_deg.abs() - tilt_config.deadzone_deg))
            .clamp(-tilt_config.max_angle_deg, tilt_config.max_angle_deg)
    };

    // Apply smoothing (simple exponential moving average would go here)

    FaceTiltData {
        yaw,
        pitch,
        roll: 0.0,
        confidence: pose.confidence,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    }
}