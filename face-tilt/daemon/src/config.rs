use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DaemonConfig {
    pub detector: DetectorConfig,
    pub pose: PoseConfig,
    pub tilt: TiltConfig,
    pub daemon: DaemonSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetectorConfig {
    pub model_path: String,
    pub input_width: u32,
    pub input_height: u32,
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub top_k: u32,
    pub backend: i32,
    pub target: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoseConfig {
    pub screen_width_mm: f32,
    pub screen_height_mm: f32,
    pub camera_offset_x_mm: f32,
    pub camera_offset_y_mm: f32,
    pub camera_offset_z_mm: f32,
    pub focal_length_mm: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TiltConfig {
    pub max_angle_deg: f32,
    pub deadzone_deg: f32,
    pub smoothing_factor: f32,
    pub invert_yaw: bool,
    pub invert_pitch: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DaemonSettings {
    pub max_fps: u32,
    pub socket_path: String,
    pub log_level: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            detector: DetectorConfig {
                model_path: "/usr/share/viewos/models/face_detection_yunet_2023mar.onnx".into(),
                input_width: 320,
                input_height: 320,
                confidence_threshold: 0.9,
                nms_threshold: 0.3,
                top_k: 5000,
                backend: opencv::dnn::DNN_BACKEND_OPENCV,
                target: opencv::dnn::DNN_TARGET_CPU,
            },
            pose: PoseConfig {
                screen_width_mm: 530.0,   // 24" 16:9
                screen_height_mm: 300.0,
                camera_offset_x_mm: 0.0,
                camera_offset_y_mm: -50.0,  // Camera above screen center
                camera_offset_z_mm: 0.0,
                focal_length_mm: 3.6,       // Typical webcam
            },
            tilt: TiltConfig {
                max_angle_deg: 12.0,
                deadzone_deg: 2.0,
                smoothing_factor: 0.15,
                invert_yaw: false,
                invert_pitch: true,
            },
            daemon: DaemonSettings {
                max_fps: 20,
                socket_path: "/run/viewos/face-tilt.sock".into(),
                log_level: "info".into(),
            },
        }
    }
}