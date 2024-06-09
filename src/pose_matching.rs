use std::cmp::min;

use crate::motion_database::{MotionDataAsset, Pose};
use bevy::prelude::*;

pub struct PoseMatchingPlugin;

impl Plugin for PoseMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, pose_motion_matching);
    }
}

pub fn pose_motion_matching(
    _asset_server: Res<AssetServer>,
    motion_assets: Res<Assets<MotionDataAsset>>,
    query: Query<&Handle<MotionDataAsset>>,
) {
    // TODO: match current tracjectory to get current pose (in runtime)
    for handle in query.iter() {
        if let Some(motion_data) = motion_assets.get(handle) {
            // Access the data in motion_data
            // println!("{:?}", motion_data);
            let poses = &motion_data.poses;
            let first_pose = poses.first().unwrap();

            for index in 1..poses.len() {
                let current_pose = poses.get(index).unwrap();
                let pose_distance = calculate_distance_summation(first_pose, current_pose);
                println!("Pose Distance Differences: {:?}", pose_distance);
            }
        }
    }
}

fn calculate_distance_summation(pose1: &Pose, pose2: &Pose) -> f32 {
    // formula: sqrt((p1-q1)^2 + (p2-q2)^2)
    let mut pose_distance: Vec<f32> = Vec::new();

    let body_part_min_len = min(pose1.len(), pose2.len());
    for body_part in 0..body_part_min_len {
        let mut body_part_distance: Vec<f32> = Vec::new();
        let body_part_pose1 = pose1.get(body_part).unwrap();
        let body_part_pose2 = pose2.get(body_part).unwrap();

        let index_min_len = min(body_part_pose1.len(), body_part_pose2.len());
        for index in 0..index_min_len {
            let p1 = body_part_pose1.get(index).unwrap_or(&0.0);
            let p2 = body_part_pose2.get(index).unwrap_or(&0.0);
            body_part_distance.push((p1 - p2).powf(2.0));
        }

        let total_body_part_distance: f32 = body_part_distance.iter().sum();
        pose_distance.push(total_body_part_distance.sqrt());
    }

    let total_post_distance: f32 = pose_distance.iter().sum();
    return total_post_distance.sqrt();
}
