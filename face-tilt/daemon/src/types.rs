//! Wire and internal data types.
//!
//! The socket format is specified in `face-tilt/shared/protocol.md`. Keep the
//! two in step: the KWin effect parses these field names verbatim.

use serde::{Deserialize, Serialize};

use opencv::core::{Point2f, Rect};

/// Protocol version. Bumped only for incompatible changes.
pub const PROTOCOL_VERSION: u32 = 1;

/// A head position update, as published on the socket.
///
/// Head *position* is sent rather than per-window angles, because the amount a
/// given window must rotate to face the viewer depends on where that window
/// is. The effect knows its own geometry; making the daemon compute angles
/// would mean shipping window positions to the daemon every frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeadPosition {
    /// Protocol version, always [`PROTOCOL_VERSION`].
    #[serde(rename = "v")]
    pub version: u32,
    /// Head position across the screen, `0.0` = left edge, `1.0` = right edge.
    pub hx: f32,
    /// Head position down the screen, `0.0` = top edge, `1.0` = bottom edge.
    pub hy: f32,
    /// Distance from the screen surface to the head, in millimetres.
    pub hz: f32,
    /// In-plane head roll in degrees. Reported for diagnostics; the effect
    /// currently ignores it.
    #[serde(rename = "roll")]
    pub roll: f32,
    /// Detection confidence, `0.0` to `1.0`. Zero means no face was found.
    #[serde(rename = "conf")]
    pub confidence: f32,
    /// Milliseconds since the Unix epoch, for staleness checks by clients.
    #[serde(rename = "t")]
    pub timestamp: i64,
}

impl Default for HeadPosition {
    fn default() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            // Centred, at a typical laptop viewing distance.
            hx: 0.5,
            hy: 0.5,
            hz: 600.0,
            roll: 0.0,
            confidence: 0.0,
            timestamp: 0,
        }
    }
}

impl HeadPosition {
    /// The head position to publish when no face has ever been seen.
    pub fn lost() -> Self {
        Self {
            confidence: 0.0,
            ..Self::default()
        }
    }
}

/// Head pose recovered from a face detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadPose {
    /// Head position relative to the screen surface, in millimetres.
    /// +X right, +Y **down** (matching screen coordinates), +Z towards the
    /// viewer.
    pub position_mm: [f32; 3],
    /// In-plane roll in degrees.
    pub roll_deg: f32,
    /// Detector confidence for the face this pose came from.
    pub confidence: f32,
}

/// One detected face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceDetection {
    /// Bounding box in camera-frame pixel coordinates.
    pub bbox: Rect,
    /// YuNet's five landmarks, in its own documented order:
    /// right eye, left eye, nose tip, right mouth corner, left mouth corner.
    pub landmarks: [Point2f; 5],
    /// Detector confidence, `0.0` to `1.0`.
    pub confidence: f32,
}

/// Milliseconds since the Unix epoch, saturating rather than panicking if the
/// system clock is set before 1970.
pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
