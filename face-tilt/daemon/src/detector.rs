use opencv::{core, dnn, prelude::*};
use crate::types::FaceDetection;

pub struct YuNetDetector {
    net: dnn::Net,
    input_size: core::Size,
    conf_threshold: f32,
    nms_threshold: f32,
    top_k: u32,
}

impl YuNetDetector {
    pub fn new(config: &crate::config::DetectorConfig) -> opencv::Result<Self> {
        let net = dnn::read_net_from_onnx(&config.model_path)?;
        net.set_preferable_backend(config.backend)?;
        net.set_preferable_target(config.target)?;

        Ok(Self {
            net,
            input_size: core::Size::new(config.input_width as i32, config.input_height as i32),
            conf_threshold: config.confidence_threshold,
            nms_threshold: config.nms_threshold,
            top_k: config.top_k,
        })
    }

    pub fn detect(&mut self, frame: &core::Mat) -> opencv::Result<Vec<FaceDetection>> {
        // Preprocess
        let mut blob = dnn::blob_from_image(
            frame,
            1.0,
            self.input_size,
            core::Scalar::default(),
            true,
            false,
            core::CV_32F,
        )?;

        self.net.set_input(&blob, "", 1.0, 0)?;

        // Forward pass
        let mut detections = core::Mat::default();
        self.net.forward(&mut detections, "")?;

        // Parse output: [1, N, 15] where 15 = x, y, w, h, confidence, 5 landmarks (x, y each)
        let detections = detections.reshape(1, 0)?;
        let rows = detections.rows();

        let mut faces = Vec::new();

        for i in 0..rows.min(self.top_k as i32) {
            let row = detections.row(i)?;
            let confidence = *row.at_2d::<f32>(0, 14)?;

            if confidence < self.conf_threshold {
                continue;
            }

            let x = *row.at_2d::<f32>(0, 0)? as i32;
            let y = *row.at_2d::<f32>(0, 1)? as i32;
            let w = *row.at_2d::<f32>(0, 2)? as i32;
            let h = *row.at_2d::<f32>(0, 3)? as i32;

            // Scale back to original frame size
            let scale_x = frame.cols() as f32 / self.input_size.width as f32;
            let scale_y = frame.rows() as f32 / self.input_size.height as f32;

            let bbox = core::Rect::new(
                (x as f32 * scale_x) as i32,
                (y as f32 * scale_y) as i32,
                (w as f32 * scale_x) as i32,
                (h as f32 * scale_y) as i32,
            );

            // Extract 5 landmarks
            let mut landmarks = [core::Point2f::default(); 5];
            for j in 0..5 {
                let lx = *row.at_2d::<f32>(0, 4 + j * 2)? * scale_x;
                let ly = *row.at_2d::<f32>(0, 5 + j * 2)? * scale_y;
                landmarks[j] = core::Point2f::new(lx, ly);
            }

            faces.push(FaceDetection {
                bbox,
                landmarks,
                confidence,
            });
        }

        // NMS (simplified - in production use cv::dnn::NMSBoxes)
        if faces.len() > 1 {
            faces.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
            let mut keep = Vec::new();
            for face in faces {
                let mut should_keep = true;
                for kept in &keep {
                    let iou = self.iou(&face.bbox, &kept.bbox);
                    if iou > self.nms_threshold {
                        should_keep = false;
                        break;
                    }
                }
                if should_keep {
                    keep.push(face);
                }
            }
            faces = keep;
        }

        Ok(faces)
    }

    fn iou(&self, a: &core::Rect, b: &core::Rect) -> f32 {
        let intersection = (a & b).area() as f32;
        let union = a.area() as f32 + b.area() as f32 - intersection;
        if union > 0.0 { intersection / union } else { 0.0 }
    }
}