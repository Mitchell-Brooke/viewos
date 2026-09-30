//! Configuration, loaded from a TOML file.
//!
//! Resolution order for the config file is in `main.rs`; this module only
//! deals with the contents.

use serde::{Deserialize, Serialize};

/// How the webcam and detector are set up.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetectorConfig {
    /// Path to the YuNet ONNX model.
    pub model_path: String,
    /// Horizontal field of view of the camera, in degrees.
    ///
    /// This is the single most important calibration value, and the one most
    /// webcams do not report. The default of 60 degrees is typical of the
    /// laptop webcams this targets; a 120 degree wide-angle USB camera needs a
    /// much larger value or head position will be badly mis-scaled.
    pub horizontal_fov_deg: f32,
    /// Aspect ratio of the frames the camera delivers (width / height). Used
    /// only to check the camera is behaving; capture is set to 640x480.
    pub expected_aspect: f32,
    /// Discard detections below this confidence.
    pub confidence_threshold: f32,
    /// Maximum number of candidate rows to read from the network output.
    pub max_candidates: usize,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            model_path: "/usr/share/viewos/models/face_detection_yunet_2023mar.onnx".to_owned(),
            horizontal_fov_deg: 60.0,
            expected_aspect: 4.0 / 3.0,
            confidence_threshold: 0.85,
            max_candidates: 5000,
        }
    }
}

/// How the camera position relative to the screen is described.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeometryConfig {
    /// Camera position relative to the centre of the screen, in millimetres.
    /// +X right, +Y **up**, +Z towards the viewer. A webcam above a laptop
    /// screen is roughly `y = +80`, `z = -200`.
    pub camera_offset_mm: [f32; 3],
    /// Assumed distance from the camera to the viewer's head, in millimetres,
    /// used when the solved pose is implausible. Also the value published
    /// before the first successful detection.
    pub nominal_viewing_distance_mm: f32,
    /// View distances further than this from nominal are treated as bad
    /// solutions and rejected, which suppresses the occasional PnP blow-up.
    pub max_viewing_distance_mm: f32,
    /// Half-width of the region of interest, as a fraction of the screen, that
    /// the head is expected to occupy. Head positions outside it are clamped.
    /// This is a robustness measure, not a tracking constraint: the effect
    /// still receives a plausible position rather than a wildly wrong one.
    pub head_search_margin: f32,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self {
            camera_offset_mm: [0.0, 80.0, -200.0],
            nominal_viewing_distance_mm: 600.0,
            max_viewing_distance_mm: 1200.0,
            head_search_margin: 0.25,
        }
    }
}

impl GeometryConfig {
    /// Closest viewing distance that is still accepted.
    ///
    /// Derived from the nominal distance rather than configured separately, so
    /// that someone who sets a nominal distance of 1500 mm for a large
    /// wall-mounted display does not also have to work out a matching floor.
    pub fn min_viewing_distance_mm(&self) -> f32 {
        (self.nominal_viewing_distance_mm * 0.25).max(150.0)
    }

    /// The `hz` value to publish before anything has been detected.
    pub fn nominal_distance_mm(&self) -> f32 {
        self.nominal_viewing_distance_mm
    }
}

/// Service-level settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonSettings {
    /// Rate at which head positions are published, in hertz.
    pub max_fps: u32,
    /// Socket path. Empty means "derive from `$XDG_RUNTIME_DIR`", which is the
    /// right answer for a user service and requires no configuration.
    pub socket_path: String,
    /// Socket permissions, as a string of octal digits.
    pub socket_mode: String,
    /// Camera index to open, for machines with more than one capture device.
    pub camera_index: i32,
    /// Capture width in pixels. The detector scales to its own input size.
    pub capture_width: i32,
    /// Capture height in pixels.
    pub capture_height: i32,
    /// Target capture rate. The publish rate is separately capped by `max_fps`.
    pub capture_fps: i32,
    /// How many consecutive frames without a face to tolerate before reporting
    /// loss. Prevents a blink from resetting the effect.
    pub lost_frame_tolerance: u32,
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            max_fps: 20,
            socket_path: String::new(),
            socket_mode: "0600".to_owned(),
            camera_index: 0,
            capture_width: 640,
            capture_height: 480,
            capture_fps: 30,
            lost_frame_tolerance: 5,
        }
    }
}

/// The whole configuration file.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonConfig {
    pub detector: DetectorConfig,
    pub geometry: GeometryConfig,
    pub daemon: DaemonSettings,
}

impl DaemonConfig {
    /// Parse a TOML document.
    ///
    /// Unknown fields are rejected rather than ignored, so that a typo in the
    /// config file is reported instead of silently doing nothing. Unknown
    /// fields are a strong sign the user is running a config from a different
    /// ViewOS version.
    pub fn from_toml(contents: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Check for values that are certainly wrong.
    ///
    /// A webcam at 60 degrees of horizontal field of view is plausible; one at
    /// zero is not, and would make the intrinsic matrix singular.
    pub fn validate(&self) -> anyhow::Result<()> {
        let fov = self.detector.horizontal_fov_deg;
        anyhow::ensure!(
            (10.0..=170.0).contains(&fov),
            "detector.horizontal_fov_deg is {fov}, which is outside the plausible range 10..170"
        );
        anyhow::ensure!(
            (0.0..=1.0).contains(&self.detector.confidence_threshold),
            "detector.confidence_threshold must be between 0 and 1"
        );
        anyhow::ensure!(
            self.daemon.max_fps > 0 && self.daemon.max_fps <= 60,
            "daemon.max_fps is {}, which is outside 1..60",
            self.daemon.max_fps
        );
        anyhow::ensure!(
            u32::from_str_radix(&self.daemon.socket_mode, 8).is_ok(),
            "daemon.socket_mode must be octal digits, e.g. 0600"
        );
        anyhow::ensure!(
            self.geometry.nominal_viewing_distance_mm > 0.0,
            "geometry.nominal_viewing_distance_mm must be positive"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_yields_defaults() {
        let config = DaemonConfig::from_toml("").expect("empty config is valid");
        assert_eq!(config.daemon.max_fps, 20);
        assert!(config.daemon.socket_path.is_empty());
    }

    #[test]
    fn partial_document_keeps_other_defaults() {
        let config = DaemonConfig::from_toml("[daemon]\nmax_fps = 30\n").unwrap();
        assert_eq!(config.daemon.max_fps, 30);
        assert_eq!(config.daemon.camera_index, 0);
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = DaemonConfig::from_toml("[daemon]\nmaxframe_rate = 30\n").unwrap_err();
        assert!(
            err.to_string().contains("maxframe_rate"),
            "error should name the offending field, got: {err}"
        );
    }

    #[test]
    fn implausible_fov_is_rejected() {
        let err = DaemonConfig::from_toml("[detector]\nhorizontal_fov_deg = 0\n").unwrap_err();
        assert!(err.to_string().contains("horizontal_fov_deg"));
    }

    #[test]
    fn non_octal_socket_mode_is_rejected() {
        let err = DaemonConfig::from_toml("[daemon]\nsocket_mode = \"rw-r--r--\"\n").unwrap_err();
        assert!(err.to_string().contains("socket_mode"));
    }

    #[test]
    fn shipped_config_file_parses() {
        // Guards against the packaged default drifting away from the schema.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/face-daemon.toml");
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        DaemonConfig::from_toml(&contents)
            .unwrap_or_else(|e| panic!("shipped config file is invalid: {e}"));
    }
}
