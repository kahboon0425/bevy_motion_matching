use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::{
        thiserror::{self, Error},
        BoxedFuture,
    },
};
use bvh_anim::{Frame, Joint};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write};

use crate::{
    bvh::{bvh_asset::BvhAsset, bvh_player::get_pose},
    player::PlayerMarker,
    trajectory::Trajectory,
    ui::BuildConfig,
};

pub struct MotionDatabasePlugin;

impl Plugin for MotionDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MotionDataAsset>()
            .init_asset_loader::<MotionDataAssetLoader>()
            .add_systems(Update, match_trajectory)
            .add_systems(Startup, load_motion_data);
    }
}

pub type Pose = Vec<Vec<f32>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct TrajectoryTransform {
    pub transform_matrix: Mat4,
    pub time: f32,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Default, Debug)]
pub struct MotionDataAsset {
    pub trajectories: Vec<TrajectoryTransform>,
    pub trajectory_offsets: Vec<usize>,
    pub joint_names: Vec<String>,
    pub joint_name_offsets: Vec<usize>,
    pub poses: Vec<Pose>,
    pub pose_offsets: Vec<usize>,
}

// trajectories: [ bvh0:traj0, bvh0:traj1, bvh0:traj2, bvh1:traj0, bvh1:traj1, bvh2:traj0 ]
//      offsets: [ 0, 3, 5, 6 ]

// joint_name_offsets output will be like this
// joint_name_offsets:[0,6,9,12,15,18,21,24,27,30,33,36,39,42,45,48,51,54,57,60,63,66,69]

impl MotionDataAsset {
    pub fn find_closest_trajectory(
        &self,
        user_trajectory: &Trajectory,
        transform: &Transform,
    ) -> Vec<f32> {
        let mut nearest_trajectories = Vec::new();

        let user_inverse_matrix = transform.compute_matrix().inverse();

        // let trajectories = self.trajectories.iter().take(7).collect::<Vec<_>>();

        for start in 0..self.trajectories.len() {
            if start + 7 > self.trajectories.len() {
                break;
            }

            let trajectories = &self.trajectories[start..start + 7];
            // println!("7 Trajectories: {:?}", trajectories);

            // Center point of trajectory
            let inv_matrix = trajectories[3].transform_matrix.inverse();

            let user_local_translations = user_trajectory
                .values
                .iter()
                .map(|user_trajectory| {
                    user_inverse_matrix.transform_point3(Vec3::new(
                        user_trajectory.x,
                        0.0,
                        user_trajectory.y,
                    ))
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let local_translations = trajectories
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(
                        trajectory
                            .transform_matrix
                            .to_scale_rotation_translation()
                            .2,
                    )
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let distance =
                calculate_trajectory_distance(&user_local_translations, &local_translations);

            nearest_trajectories.push(distance);

            // println!("Distance: {} Index:{}", distance, i);
        }

        println!("List before sort: {:?}", nearest_trajectories);
        nearest_trajectories.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("List after sort: {:?}", nearest_trajectories);

        if nearest_trajectories.len() > 10 {
            nearest_trajectories.truncate(10)
        }

        nearest_trajectories
    }
}

pub fn calculate_trajectory_distance(t1: &[Vec2], t2: &[Vec2]) -> f32 {
    // distance = sqrt((p1-q1)^2 + (p2-q2)^2)
    t1.iter()
        .zip(t2.iter())
        .map(|(p, traj)| (*p - *traj).length_squared())
        .sum::<f32>()
}

pub fn match_trajectory(
    motion_data_assets: Res<Assets<MotionDataAsset>>,
    query_motion_data: Query<&Handle<MotionDataAsset>>,
    user_input_trajectory: Query<(&Trajectory, &Transform), With<PlayerMarker>>,
) {
    for handle in query_motion_data.iter() {
        if let Some(motion_data) = motion_data_assets.get(handle) {
            for (trajectory, transform) in user_input_trajectory.iter() {
                let nearest_trajectory = motion_data.find_closest_trajectory(trajectory, transform);
                println!("10 nearest trajectory: {:?}", nearest_trajectory);
            }
        }
    }
}

fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load::<MotionDataAsset>("motion_data/motion_data.json");
    commands.spawn(handle);
}

#[derive(Default)]
struct MotionDataAssetLoader;

impl AssetLoader for MotionDataAssetLoader {
    type Asset = MotionDataAsset;
    type Settings = ();
    type Error = MotionDataLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();

            reader.read_to_end(&mut bytes).await?;

            let motion_data: MotionDataAsset = serde_json::from_slice(&bytes)?;

            let motion_data_asset = MotionDataAsset {
                trajectories: motion_data.trajectories,
                trajectory_offsets: motion_data.trajectory_offsets,
                poses: motion_data.poses,
                pose_offsets: motion_data.pose_offsets,
                joint_names: motion_data.joint_names,
                joint_name_offsets: motion_data.joint_name_offsets,
            };

            Ok(motion_data_asset)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MotionDataLoaderError {
    #[error("Could not load json file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not deserialize using serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn extract_motion_data(bvh_asset: &Assets<BvhAsset>, build_config: &mut BuildConfig) {
    let mut motion_data = MotionDataAsset::default();

    let mut trajectory_data_len = 0;
    let mut motion_data_len = 0;
    let mut joint_name_offsets = Vec::new();
    let mut joint_name_data_len = 0;

    for id in build_config.bvh_assets.iter() {
        let Some(BvhAsset(bvh)) = bvh_asset.get(*id) else {
            return;
        };

        let interval = 0.3333;
        let frame_count = bvh.frames().len();
        let total_duration = bvh.frame_time().as_secs_f32() * frame_count as f32;

        motion_data.trajectory_offsets.push(trajectory_data_len);
        let mut trajectory_index = 0;
        loop {
            let time = interval * trajectory_index as f32;
            if time > total_duration {
                break;
            }
            let (frame_index, _interp_factor) = get_pose(time, bvh);

            if let Some(future_frame) = bvh.frames().nth(frame_index) {
                if let Some(hip_joint) = bvh.joints().find(|j| j.data().name() == "Hips") {
                    let translation = get_joint_position(&hip_joint, future_frame) * 0.01;
                    let euler_angle = get_joint_euler_angle(&hip_joint, future_frame);
                    let rotation = Quat::from_euler(
                        EulerRot::XYZ,
                        euler_angle.x,
                        euler_angle.y,
                        euler_angle.z,
                    );
                    let transform_matrix = Mat4::from_rotation_translation(rotation, translation);

                    trajectory_data_len += 1;
                    motion_data.trajectories.push(TrajectoryTransform {
                        transform_matrix,
                        time,
                    });
                }
            }
            trajectory_index += 1;
        }

        if motion_data.joint_names.is_empty() {
            for joint in bvh.joints() {
                motion_data
                    .joint_names
                    .push(joint.data().name().to_string());
                joint_name_offsets.push(joint_name_data_len);
                joint_name_data_len += joint.data().channels().len();
            }
            joint_name_offsets.push(joint_name_data_len);
        }

        motion_data.pose_offsets.push(motion_data_len);
        motion_data_len += bvh.num_frames();

        motion_data.poses.push(
            bvh.frames()
                .map(|f| f.as_slice().to_owned())
                .collect::<Vec<_>>(),
        );
    }

    motion_data.trajectory_offsets.push(trajectory_data_len);
    motion_data.pose_offsets.push(motion_data_len);
    motion_data.joint_name_offsets = joint_name_offsets;

    // TODO(perf): Serialize into binary instead
    let convert_to_json = serde_json::to_string(&motion_data).unwrap();

    let mut motion_library = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        // TODO: specify a file name and possibly a  location
        .open("assets/motion_data/motion_data.json")
        .unwrap();

    motion_library
        .write_all(convert_to_json.as_bytes())
        .unwrap();
}

pub fn get_joint_position(joint: &Joint, frame: &Frame) -> Vec3 {
    let channels = joint.data().channels();
    let x = frame[&channels[0]];
    // let y = frame[&channels[1]];
    let z = frame[&channels[2]];
    Vec3::new(x, 0.0, z)
}

pub fn get_joint_euler_angle(joint: &Joint, frame: &Frame) -> Vec3 {
    let channels = joint.data().channels();
    let z = frame[&channels[3]];
    let y = frame[&channels[4]];
    let x = frame[&channels[5]];
    Vec3::new(x, y, z)
}
