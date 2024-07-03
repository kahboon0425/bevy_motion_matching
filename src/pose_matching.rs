use std::cmp::min;

use crate::motion_database::{MotionDataAsset, Pose};
use bevy::prelude::*;

pub fn match_pose(
    motion_data_assets: &Res<Assets<MotionDataAsset>>,
    query_motion_data: &Query<&Handle<MotionDataAsset>>,
    nearest_poses: Vec<&Pose>,
) {
    // TODO: match current tracjectory to get current pose (in runtime)
    for handle in query_motion_data.iter() {
        if let Some(motion_data) = motion_data_assets.get(handle) {
            // Access the data in motion_data
            // println!("{:?}", motion_data);

            for p in 0..nearest_poses.len() {
                let nearest_pose = nearest_poses.get(p).unwrap();
                let current_pose = motion_data.poses.first().unwrap();
                let distance = get_distance(nearest_pose, current_pose);
                println!("Pose Distance for trajectory #{:?}: {:?}", p, distance);
            }
        }
    }
}

fn get_distance(p1: &Pose, p2: &Pose) -> f32 {
    // formula: sqrt((p1-q1)^2 + (p2-q2)^2)
    let mut distance: Vec<f32> = Vec::new();
    for joint in 0..min(p1.len(), p2.len()) {
        let joint1 = p1.get(joint).unwrap();
        let joint2 = p2.get(joint).unwrap();
        distance.push((joint1 - joint2).powf(2.0));
    }
    let total_distance: f32 = distance.iter().sum();
    return total_distance.sqrt();
}
