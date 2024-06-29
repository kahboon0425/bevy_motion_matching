use bevy::{prelude::*, utils::HashMap};

#[derive(Asset, TypePath, Default, Debug, Clone)]
pub struct BvhConfigAsset(HashMap<String, JointConfig>);

#[derive(Default, Debug, Clone, Copy)]
pub struct JointConfig {
    pub rotation_offset: Vec3,
    pub rotation_inverse: BVec3,
    pub position_offset: Vec3,
    pub position_inverse: BVec3,
}
