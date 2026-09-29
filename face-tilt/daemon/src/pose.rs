use nalgebra::{Matrix3, Point3, Vector3};
use opencv::{core, calib3d, prelude::*};
use crate::types::{FaceDetection, HeadPose};

pub struct HeadPoseSolver {
    // 3D model points for the 5 facial landmarks (in mm, relative to face center)
    // Using a canonical face model
    model_points: Vec<Point3<f32>>,
    camera_matrix: core::Mat,
    dist_coeffs: core::Mat,
    screen_width_mm: f32,
    screen_height_mm: f32,
    camera_offset: Vector3<f32>,
}

impl HeadPoseSolver {
    pub fn new(config: &crate::config::PoseConfig) -> opencv::Result<Self> {
        // Canonical 3D face model points (mm) for 5 landmarks:
        // 0: left eye corner, 1: right eye corner, 2: nose tip, 3: left mouth corner, 4: right mouth corner
        let model_points = vec![
            Point3::new(-35.0, -30.0, -10.0),  // Left eye
            Point3::new(35.0, -30.0, -10.0),   // Right eye
            Point3::new(0.0, 0.0, 10.0),       // Nose tip
            Point3::new(-25.0, 30.0, -5.0),    // Left mouth
            Point3::new(25.0, 30.0, -5.0),     // Right mouth
        ];

        // Camera intrinsics (simplified - should come from calibration)
        let fx = config.focal_length_mm * 100.0; // Approximate pixels/mm
        let fy = fx;
        let cx = config.screen_width_mm / 2.0;
        let cy = config.screen_height_mm / 2.0;

        let camera_matrix = core::Mat::from_slice(&[
            fx, 0.0, cx,
            0.0, fy, cy,
            0.0, 0.0, 1.0
        ])?.reshape(1, 3)?.to_mat()?;

        let dist_coeffs = core::Mat::zeros(5, 1, core::CV_64F)?;

        let camera_offset = Vector3::new(
            config.camera_offset_x_mm,
            config.camera_offset_y_mm,
            config.camera_offset_z_mm,
        );

        Ok(Self {
            model_points,
            camera_matrix,
            dist_coeffs,
            screen_width_mm: config.screen_width_mm,
            screen_height_mm: config.screen_height_mm,
            camera_offset,
        })
    }

    pub fn solve(&self, face: &FaceDetection, frame: &core::Mat) -> opencv::Result<HeadPose> {
        // Prepare 2D image points from landmarks
        let mut image_points = core::Mat::zeros(5, 2, core::CV_32F)?;
        for (i, landmark) in face.landmarks.iter().enumerate() {
            *image_points.at_2d_mut::<f32>(i, 0)? = landmark.x;
            *image_points.at_2d_mut::<f32>(i, 1)? = landmark.y;
        }

        // Prepare 3D model points
        let mut object_points = core::Mat::zeros(5, 3, core::CV_32F)?;
        for (i, pt) in self.model_points.iter().enumerate() {
            *object_points.at_2d_mut::<f32>(i, 0)? = pt.x;
            *object_points.at_2d_mut::<f32>(i, 1)? = pt.y;
            *object_points.at_2d_mut::<f32>(i, 2)? = pt.z;
        }

        // Solve PnP
        let mut rvec = core::Mat::default();
        let mut tvec = core::Mat::default();

        calib3d::solve_pnp(
            &object_points,
            &image_points,
            &self.camera_matrix,
            &self.dist_coeffs,
            &mut rvec,
            &mut tvec,
            false,
            calib3d::SOLVEPNP_ITERATIVE,
        )?;

        // Convert rotation vector to Euler angles
        let mut rmat = core::Mat::default();
        calib3d::rodrigues(&rvec, &mut rmat)?;

        // Extract Euler angles (yaw, pitch, roll) from rotation matrix
        // OpenCV uses: X=pitch, Y=yaw, Z=roll (but order matters)
        let rmat_f32: Matrix3<f32> = rmat.try_into().unwrap();

        // yaw around Y, pitch around X, roll around Z
        let yaw = rmat_f32[(1, 0)].atan2(rmat_f32[(0, 0)]);
        let pitch = (-rmat_f32[(2, 0)]).asin();
        let roll = rmat_f32[(2, 1)].atan2(rmat_f32[(2, 2)]);

        // Camera-relative position
        let tvec_f32: Vector3<f32> = Vector3::new(
            *tvec.at_2d::<f64>(0, 0)? as f32,
            *tvec.at_2d::<f64>(1, 0)? as f32,
            *tvec.at_2d::<f64>(2, 0)? as f32,
        );

        // Transform to screen-relative coordinates
        let position = Point3::from(tvec_f32 + self.camera_offset);

        Ok(HeadPose {
            yaw,
            pitch,
            roll,
            position,
            confidence: face.confidence,
        })
    }
}