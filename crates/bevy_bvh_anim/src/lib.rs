pub use bvh_anim;

pub mod prelude {
    pub use crate::bvh_asset::{BvhAsset, BvhAssetPlugin};
    pub use crate::joint_matrices::JointMatrices;
    // Re-exports bvh_anim's commonly used types
    pub use bvh_anim::{
        bvh, Axis as BvhAxis, Bvh, Channel, Frame, Frames, Joint, JointData, JointName,
    };
}
pub mod bvh_asset;
pub mod joint_matrices;
