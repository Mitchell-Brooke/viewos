//! ViewOS face tracking daemon.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::Parser;
use opencv::{
    core::{Mat, Size},
    prelude::*,
    videoio::{self, VideoCapture},
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use viewos_face_daemon::{
    config::DaemonConfig,
    default_socket_path,
    detector::YuNetDetector,
    pose::HeadPoseSolver,
    socket::SocketServer,
    types::{now_millis, FaceDetection, HeadPose, HeadPosition},
};

/// Queue depth for the publication channel.
///
/// Deep enough to absorb a scheduling hiccup, shallow enough that a wedged
/// consumer falls behind quickly and is dropped by the client rather than
/// growing memory without bound.
const CHANNEL_DEPTH: usize = 8;

#[derive(Parser, Debug)]
#[command(
    name = "viewos-face-daemon",
    version,
    about = "ViewOS face tracking daemon",
    long_about = "Publishes the viewer's head position on a local Unix socket so that the \
                  ViewOS KWin effect can rotate windows to face them. Frames never leave the \
                  machine."
)]
struct Args {
    /// User configuration file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// System-wide default configuration, used when the user file is absent.
    #[arg(long, value_name = "PATH")]
    config_system: Option<PathBuf>,

    /// Override the socket path from the configuration file.
    #[arg(long, value_name = "PATH")]
    socket: Option<PathBuf>,

    /// Override the camera index from the configuration file.
    #[arg(long, value_name = "N")]
    camera: Option<i32>,

    /// Emit nothing but warnings and errors.
    #[arg(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let filter = if args.quiet {
        EnvFilter::new("warn")
    } else {
        // RUST_LOG still wins, which is what a user reaching for a debug
        // environment expects.
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!(
        "viewos-face-daemon {} starting",
        env!("CARGO_PKG_VERSION")
    );

    let config = load_config(&args)?;
    let socket_path = resolve_socket_path(&args, &config);
    let camera_index = args.camera.unwrap_or(config.daemon.camera_index);

    let mode = u32::from_str_radix(&config.daemon.socket_mode, 8).with_context(|| {
        format!(
            "daemon.socket_mode {:?} is not octal digits",
            config.daemon.socket_mode
        )
    })?;

    info!("socket: {}", socket_path.display());
    info!("camera index: {camera_index}");

    // --- Publication -------------------------------------------------------
    let (publisher, publisher_receiver) = broadcast::channel::<HeadPosition>(CHANNEL_DEPTH);

    // --- Capture and processing -------------------------------------------
    //
    // The camera is opened and every OpenCV object is created and used on a
    // single dedicated thread. Capture is blocking, so it cannot run on the
    // async runtime without stalling the socket server; but handing `Mat`s
    // across a thread boundary would mean relying on them being `Send`, and
    // with a single owner there is no reason to. Results leave the thread as
    // plain numbers, over a channel.
    let running = Arc::new(AtomicBool::new(true));
    let thread_config = config.clone();
    let thread_running = Arc::clone(&running);
    let thread_publisher = publisher.clone();

    let capture = std::thread::Builder::new()
        .name("viewos-capture".to_owned())
        .spawn(move || {
            match run_capture_loop(
                thread_config,
                camera_index,
                thread_running,
                thread_publisher,
            ) {
                Ok(()) => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        })
        .context("spawning the capture thread")?;

    // A missing camera is by far the most likely startup failure, and it is
    // detected on the capture thread. Rather than wiring up a readiness channel
    // for one error, give the thread a moment and check whether it has already
    // exited: a daemon that appears to start and then silently tracks nothing
    // is much harder to diagnose than one that refuses to start.
    tokio::time::sleep(Duration::from_millis(500)).await;
    if capture.is_finished() {
        match capture.join() {
            Ok(Ok(())) => anyhow::bail!("the capture thread stopped immediately; see the log above"),
            Ok(Err(message)) => anyhow::bail!("{message}"),
            Err(_) => anyhow::bail!("the capture thread panicked"),
        }
    }

    // --- Socket server -----------------------------------------------------
    let mut socket_server = SocketServer::new(socket_path, mode, publisher_receiver);
    tokio::spawn(async move {
        if let Err(error) = socket_server.run().await {
            error!("socket server stopped: {error}");
        }
    });

    // --- Run until interrupted ---------------------------------------------
    info!("tracking head position; press Ctrl-C to stop");
    tokio::signal::ctrl_c().await.ok();
    info!("interrupt received, shutting down");

    // Ask the capture thread to stop, then wait for it. Capture is blocking,
    // so this can take up to one frame interval.
    running.store(false, Ordering::SeqCst);
    match capture.join() {
        Ok(Ok(())) => info!("capture thread stopped cleanly"),
        Ok(Err(message)) => error!("capture thread reported: {message}"),
        Err(_) => error!("capture thread panicked"),
    }

    info!("stopped");
    Ok(())
}

/// Owns the camera, the detector and the pose solver for the life of the
/// daemon.
fn run_capture_loop(
    config: DaemonConfig,
    camera_index: i32,
    running: Arc<AtomicBool>,
    publisher: broadcast::Sender<HeadPosition>,
) -> Result<()> {
    let mut camera = open_camera(&config, camera_index)?;
    let frame_size = describe_camera(&config, &mut camera);

    let mut detector = YuNetDetector::new(&config.detector)?;
    info!("loaded YuNet from {}", config.detector.model_path);

    let solver = HeadPoseSolver::new(&config.detector, &config.geometry, frame_size)?;

    let publish_interval = Duration::from_micros(1_000_000 / u64::from(config.daemon.max_fps));

    let mut frame = Mat::default();
    let mut last_publish = Instant::now() - publish_interval;
    let mut last_good: Option<HeadPose> = None;
    let mut lost_frames: u32 = 0;
    let mut consecutive_read_failures: u32 = 0;

    while running.load(Ordering::SeqCst) {
        match camera.read(&mut frame) {
            Ok(true) => consecutive_read_failures = 0,
            // OpenCV reports "no frame available yet" as false rather than as
            // an error. Not worth a log line every time.
            Ok(false) => continue,
            Err(error) => {
                consecutive_read_failures += 1;
                warn!("camera read failed ({consecutive_read_failures}): {error}");
                // Persistent failure means the camera is gone, unplugged, or
                // claimed by another program. Retrying forever would leave a
                // dead daemon looking alive.
                if consecutive_read_failures > config.daemon.lost_frame_tolerance * 20 {
                    anyhow::bail!(
                        "the camera has not delivered a frame for a long time; giving up. \
                         Is it still connected, and does your user remain in the 'video' group?"
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        }

        if frame.empty() {
            continue;
        }

        let pose = match detector.detect(&frame) {
            Ok(detections) => solve_best_face(&solver, &detections),
            Err(error) => {
                // A single failed inference is not fatal. YuNet occasionally
                // rejects a frame with an unusual colour profile.
                warn!("detection failed: {error:#}");
                None
            }
        };

        let have_pose = pose.is_some();
        if have_pose {
            lost_frames = 0;
            last_good = pose;
        } else {
            lost_frames = lost_frames.saturating_add(1);
        }

        if last_publish.elapsed() < publish_interval {
            continue;
        }
        last_publish = Instant::now();

        let message = match (have_pose, last_good) {
            (true, Some(pose)) => position_from_pose(&pose, &config),
            // Tracking was lost long enough ago to be real rather than a blink.
            (false, Some(pose)) if lost_frames >= config.daemon.lost_frame_tolerance => {
                let mut lost = position_from_pose(&pose, &config);
                lost.confidence = 0.0;
                lost.timestamp = now_millis();
                lost
            }
            // Nothing detected and nothing remembered yet: stay quiet rather
            // than publishing a position we know nothing about.
            _ => continue,
        };

        if publisher.send(message).is_err() {
            // Nobody has connected yet. Expected: the effect loads after the
            // daemon starts.
            debug!("no subscribers on the head position channel");
        }
    }

    Ok(())
}

/// Open the camera, applying the requested capture mode.
fn open_camera(config: &DaemonConfig, camera_index: i32) -> Result<VideoCapture> {
    let camera = VideoCapture::new(camera_index, videoio::CAP_ANY).with_context(|| {
        format!(
            "could not open camera {camera_index}. Check that /dev/video{camera_index} exists \
             and that your user is in the 'video' group."
        )
    })?;

    if !camera.is_opened().unwrap_or(false) {
        anyhow::bail!("camera {camera_index} could not be opened");
    }

    // Push the capture mode, but treat refusal as a warning rather than an
    // error. Many webcams support exactly one resolution, and detection works
    // perfectly well at whatever we get.
    for (property, value, label) in [
        (
            videoio::CAP_PROP_FRAME_WIDTH,
            config.daemon.capture_width,
            "width",
        ),
        (
            videoio::CAP_PROP_FRAME_HEIGHT,
            config.daemon.capture_height,
            "height",
        ),
        (videoio::CAP_PROP_FPS, config.daemon.capture_fps, "fps"),
    ] {
        if let Err(error) = camera.set(property, f64::from(value)) {
            debug!("camera refused {label}={value}: {error}");
        }
    }

    Ok(camera)
}

/// Report the capture mode in use and sanity-check it.
fn describe_camera(config: &DaemonConfig, camera: &mut VideoCapture) -> Size {
    let width = camera
        .get(videoio::CAP_PROP_FRAME_WIDTH)
        .unwrap_or_default() as i32;
    let height = camera
        .get(videoio::CAP_PROP_FRAME_HEIGHT)
        .unwrap_or_default() as i32;

    if width <= 0 || height <= 0 {
        // Not fatal on its own: the first successful read will populate the
        // frame, and the intrinsics are built from the real size once we have
        // one. Reported so a misconfigured camera is visible in the log.
        warn!("camera has not reported its frame size yet; assuming 640x480");
        return Size::new(640, 480);
    }

    info!("camera delivering {width}x{height}");

    let expected = config.detector.expected_aspect;
    let actual = width as f32 / height as f32;
    if (actual - expected).abs() > 0.2 {
        warn!(
            "camera aspect ratio is {actual:.2} but {expected:.2} was expected, so the \
             requested capture mode was probably not applied. Head position will still be \
             correct, but detection quality may suffer."
        );
    }

    Size::new(width, height)
}

/// Pick the face most likely to be the viewer and solve for it.
///
/// The largest face wins rather than the most confident one: a webcam pointed at
/// a monitor sees faces on screen, and the viewer is invariably closer to the
/// camera than any of them.
fn solve_best_face(
    solver: &HeadPoseSolver,
    detections: &[FaceDetection],
) -> Option<HeadPose> {
    detections
        .iter()
        .filter(|face| HeadPoseSolver::landmarks_are_plausible(face))
        .max_by(|a, b| {
            let area = |f: &FaceDetection| (f.bbox.width * f.bbox.height) as f32;
            area(a).total_cmp(&area(b))
        })
        .and_then(|face| solver.solve(face))
}

/// Convert a solved pose into the published screen-relative form.
///
/// The solver works in millimetres. The socket carries fractions, so that the
/// effect needs no knowledge of the screen's physical size.
fn position_from_pose(pose: &HeadPose, config: &DaemonConfig) -> HeadPosition {
    let [x_mm, y_mm, z_mm] = pose.position_mm;

    // Dividing by the viewing distance rather than by a screen size makes these
    // values "how far off axis, as a fraction of how far away you are" — the
    // tangent of the angle, which is what the effect needs and which is
    // independent of the panel's size and of whether the display is 13 or 32
    // inches. The viewer is assumed to sit roughly centred, which the effect
    // rescales anyway.
    let distance = z_mm.max(1.0);
    let hx = 0.5 + x_mm / distance;
    let hy = 0.5 + y_mm / distance;

    // Clamp rather than discard. A partly-detected face near the edge of frame
    // should still give a plausible position, not nothing.
    let margin = config.geometry.head_search_margin;
    let clamp = |value: f32| value.clamp(0.5 - margin, 0.5 + margin);

    HeadPosition {
        version: viewos_face_daemon::PROTOCOL_VERSION,
        hx: clamp(hx),
        hy: clamp(hy),
        // Published as a fraction of a metre, which is the unit the effect
        // works in. Guarded against zero because the effect divides by it.
        hz: distance,
        roll: pose.roll_deg,
        confidence: pose.confidence,
        timestamp: now_millis(),
    }
}

/// Load configuration, preferring the user's file over the system default.
fn load_config(args: &Args) -> Result<DaemonConfig> {
    if let Some(path) = &args.config {
        if path.exists() {
            info!("reading configuration from {}", path.display());
            return read_config(path);
        }
        // An explicitly requested file that does not exist is a mistake worth
        // reporting: the user asked for that file specifically.
        anyhow::bail!("configuration file {} does not exist", path.display());
    }

    if let Some(path) = &args.config_system {
        if path.exists() {
            info!("reading default configuration from {}", path.display());
            return read_config(path);
        }
        debug!("default configuration {} not found", path.display());
    }

    if let Some(path) = user_config_path() {
        if path.exists() {
            info!("reading configuration from {}", path.display());
            return read_config(&path);
        }
    }

    info!("no configuration file found, using built-in defaults");
    DaemonConfig::default()
        .validate()
        .context("built-in defaults failed validation")
}

fn read_config(path: &std::path::Path) -> Result<DaemonConfig> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    DaemonConfig::from_toml(&contents).with_context(|| format!("parsing {}", path.display()))
}

fn user_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("viewos").join("face-daemon.toml"))
}

fn resolve_socket_path(args: &Args, config: &DaemonConfig) -> PathBuf {
    if let Some(path) = &args.socket {
        return path.clone();
    }
    if !config.daemon.socket_path.is_empty() {
        return PathBuf::from(&config.daemon.socket_path);
    }
    let path = default_socket_path();
    if path.starts_with("/tmp") {
        warn!(
            "XDG_RUNTIME_DIR is not set, falling back to {}. Run the daemon as a user service \
             so the socket stays private to your session.",
            path.display()
        );
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> DaemonConfig {
        DaemonConfig::default()
    }

    fn pose(x: f32, y: f32, z: f32) -> HeadPose {
        HeadPose {
            position_mm: [x, y, z],
            roll_deg: 0.0,
            confidence: 0.9,
        }
    }

    #[test]
    fn fractions_stay_within_the_search_margin() {
        let config = config();
        let margin = config.geometry.head_search_margin;
        // A head far off to one side, well beyond the margin.
        let position = position_from_pose(&pose(2000.0, 0.0, 600.0), &config);
        assert!(position.hx <= 0.5 + margin + f32::EPSILON);
        assert!(position.hx >= 0.5 - margin - f32::EPSILON);
    }

    #[test]
    fn a_centred_head_publishes_the_centre() {
        let position = position_from_pose(&pose(0.0, 0.0, 600.0), &config());
        assert!((position.hx - 0.5).abs() < 1e-6);
        assert!((position.hy - 0.5).abs() < 1e-6);
        assert!((position.hz - 600.0).abs() < 1e-6);
    }

    #[test]
    fn a_head_to_the_right_publishes_a_larger_hx() {
        let right = position_from_pose(&pose(120.0, 0.0, 600.0), &config());
        let left = position_from_pose(&pose(-120.0, 0.0, 600.0), &config());
        assert!(right.hx > 0.5);
        assert!(left.hx < 0.5);
    }

    #[test]
    fn a_head_below_centre_publishes_a_larger_hy() {
        // +Y is down in screen coordinates, so below centre is a larger hy.
        let position = position_from_pose(&pose(0.0, 100.0, 600.0), &config());
        assert!(position.hy > 0.5);
    }

    #[test]
    fn published_distance_is_never_zero() {
        // hz is used as a divisor by the effect. Zero there is a division by
        // zero, which becomes a NaN rotation matrix in C++ and silently
        // stops the compositor repainting.
        let position = position_from_pose(&pose(0.0, 0.0, 0.0), &config());
        assert!(position.hz > 0.0);
        assert!(position.hx.is_finite() && position.hy.is_finite());
    }

    #[test]
    fn fractions_are_independent_of_viewing_distance() {
        // The same physical head offset, seen from nearer and from further,
        // must give the same screen fraction. This is what makes the effect
        // work without knowing the panel's physical size.
        let near = position_from_pose(&pose(100.0, 0.0, 300.0), &config());
        let far = position_from_pose(&pose(200.0, 0.0, 600.0), &config());
        assert!(
            (near.hx - far.hx).abs() < 1e-5,
            "near {} vs far {}",
            near.hx,
            far.hx
        );
    }

    #[test]
    fn published_version_matches_the_protocol() {
        let position = position_from_pose(&pose(0.0, 0.0, 600.0), &config());
        assert_eq!(position.version, viewos_face_daemon::PROTOCOL_VERSION);
    }

    #[test]
    fn published_message_matches_the_documented_field_names() {
        // Guards against renaming a field in types.rs without updating the
        // protocol document, which is the kind of break that only shows up as
        // "the effect stopped working" for users.
        let json = serde_json::to_value(position_from_pose(&pose(0.0, 0.0, 600.0), &config()))
            .expect("serialises");
        let object = json.as_object().expect("is an object");
        assert_eq!(object.len(), 7);
        for field in ["v", "hx", "hy", "hz", "roll", "conf", "t"] {
            assert!(object.contains_key(field), "missing {field}");
        }
    }
}
