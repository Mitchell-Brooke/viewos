use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FaceTiltData {
    pub yaw: f32,       // Rotation around Y axis (degrees)
    pub pitch: f32,     // Rotation around X axis (degrees)
    pub roll: f32,      // Rotation around Z axis (degrees)
    pub confidence: f32, // Detection confidence 0.0-1.0
    pub timestamp: u64,  // Milliseconds since epoch
}

impl Default for FaceTiltData {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            confidence: 0.0,
            timestamp: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadPose {
    pub yaw: f32,       // Radians
    pub pitch: f32,     // Radians
    pub roll: f32,      // Radians
    pub position: nalgebra::Point3<f32>, // Camera-relative position
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceDetection {
    pub bbox: opencv::core::Rect,
    pub landmarks: [opencv::core::Point2f; 5], // eye corners, nose, mouth corners
    pub confidence: f32,
}

impl FaceDetection {
    pub fn center(&self) -> opencv::core::Point2f {
        opencv::core::Point2f::new(
            self.bbox.x as f32 + self.bbox.width as f32 / 2.0,
            self.bbox.y as f32 + self.bbox.height as f32 / 2.0,
        )
    }
}