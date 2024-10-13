use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(super) struct MotionDataAssetPlugin;

impl Plugin for MotionDataAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MotionDataAsset>()
            .init_asset_loader::<MotionDataAssetLoader>();
    }
}

// TODO: Private data structures inside the asset to ensure data integrity.

/// A memory and storage efficient storage of [`JointInfo`] and multiple motion data ([`Trajectories`] & [`Poses`]).
#[derive(Asset, TypePath, Serialize, Deserialize, Debug)]
pub struct MotionDataAsset {
    /// Joint data.
    joints: Vec<JointInfo>,
    /// Trajectory data for trajectory matching.
    pub trajectories: Trajectories,
    /// Pose data for pose matching and animation sampling.
    pub poses: Poses,
}

impl MotionDataAsset {
    pub fn new(bvh: &Bvh, trajectory_interval: f32) -> Self {
        Self {
            joints: bvh
                .joints()
                .map(|j| JointInfo::from_joint_data(j.data()))
                .collect(),
            trajectories: Trajectories::new(trajectory_interval),
            poses: Poses::new(bvh.frame_time().as_secs_f32()),
        }
    }

    pub fn append_frames(&mut self, bvh: &Bvh) {
        let bvh_frame_time = bvh.frame_time().as_secs_f32();
        if bvh_frame_time != self.poses.interval {
            error!(
                "Bvh frame time ({}) does not match pose interval ({}).",
                bvh_frame_time, self.poses.interval
            );
            return;
        }

        self.poses.append_frames(bvh);
        self.trajectories.append_frames(bvh);
    }
}

impl MotionDataAsset {
    pub fn joints(&self) -> &[JointInfo] {
        &self.joints
    }

    pub fn get_joint(&self, index: usize) -> Option<&JointInfo> {
        self.joints.get(index)
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Deref, DerefMut)]
pub struct Pose(pub Vec<f32>);

impl Pose {
    pub fn from_frame(frame: &Frame) -> Self {
        Self(frame.as_slice().to_vec())
    }

    /// Get position and rotation.
    pub fn get_pos_rot(&self, joint_info: &JointInfo) -> (Vec3, Quat) {
        let mut pos = Vec3::ZERO;
        let mut euler = Vec3::ZERO;
        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::RotationX => euler.x = self[i].to_radians(),
                ChannelType::RotationY => euler.y = self[i].to_radians(),
                ChannelType::RotationZ => euler.z = self[i].to_radians(),
                ChannelType::PositionX => pos.x = self[i],
                ChannelType::PositionY => pos.y = self[i],
                ChannelType::PositionZ => pos.z = self[i],
            }
        }

        (
            pos,
            Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z),
        )
    }
}

#[inline]
fn frame_to_pose(frame: &Frame) -> Pose {
    Pose(frame.as_slice().to_vec())
}

/// Stores chunks of poses.
#[derive(Serialize, Deserialize, Debug)]
pub struct Poses {
    /// Pose data that can be sampled using [`JointInfo`].
    poses: Vec<Pose>,
    /// Offset index of [`Self::poses`] chunks.
    ///
    /// # Example
    ///
    /// \[0, 3, 5, 7\] contains chunk [0, 3), [3, 5), [5, 7)
    offsets: Vec<usize>,
    /// Duration between each pose in seconds.
    interval: f32,
}

impl Poses {
    pub fn new(interval: f32) -> Self {
        assert!(
            interval > 0.0,
            "Interval time between poses must be greater than 0!"
        );

        Self {
            poses: vec![],
            offsets: vec![0],
            interval,
        }
    }

    pub fn append_frames(&mut self, bvh: &Bvh) {
        let frames = bvh.frames();
        self.offsets
            .push(self.offsets[self.offsets.len() - 1] + frames.len());

        for frame in frames {
            self.poses.push(frame_to_pose(frame));
        }
    }

    /// Get poses of a particular chunk.
    pub fn get_poses_from_chunk(&self, chunk_index: usize) -> &[Pose] {
        let start_index = self.offsets[chunk_index];
        let end_index = self.offsets[chunk_index + 1];

        &self.poses[start_index..end_index]
    }

    /// Calculate the time value from a chunk offset index.
    pub fn time_from_chunk_offset(&self, chunk_offset: usize) -> f32 {
        chunk_offset as f32 * self.interval
    }

    /// Calculate the floored chunk offset index from a time value.
    pub fn chunk_offset_from_time(&self, time: f32) -> usize {
        (time / self.interval) as usize
    }
}

// Getter functions
impl Poses {
    pub fn poses(&self) -> &[Pose] {
        &self.poses
    }

    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    pub fn interval(&self) -> f32 {
        self.interval
    }
}

/// Stores chunks of trajectory matrices.
#[derive(Serialize, Deserialize, Debug)]
pub struct Trajectories {
    /// Trajectory matrices.
    matrices: Vec<Mat4>,
    /// Offset index of [`Self::matrices`] chunks.
    ///
    /// # Example
    ///
    /// \[0, 3, 5, 7\] contains chunk [0, 3), [3, 5), [5, 7)
    offsets: Vec<usize>,
    /// Duration between each trajectory matrix in seconds.
    interval: f32,
}

impl Trajectories {
    pub fn new(interval: f32) -> Self {
        assert!(
            interval > 0.0,
            "Interval time between trajectories must be greater than 0!"
        );

        Self {
            matrices: vec![],
            offsets: vec![0],
            interval,
        }
    }

    fn append_frames(&mut self, bvh: &Bvh) {
        let frame_count = bvh.frames().len();
        let frame_time = bvh.frame_time().as_secs_f32();
        // SAFETY: A root joint is expected to be present in the Bvh
        let root_joint = bvh.root_joint().unwrap();

        let total_frame_time = frame_count as f32 * frame_time;
        let trajectory_count = (total_frame_time / self.interval) as usize;

        self.offsets
            .push(self.offsets[self.offsets.len() - 1] + trajectory_count);

        for t in 0..trajectory_count {
            let time = t as f32 * self.interval;

            // Interpolate between start and end frame
            let start_frame_index = f32::floor(time / frame_time) as usize;
            let end_frame_index =
                usize::min(f32::ceil(time / frame_time) as usize, frame_count - 1);

            // Calculation above should made sure that both start & end frame index
            // is within the bounds of frame count.
            let Some(start_frame) = bvh.frames().nth(start_frame_index) else {
                error!("Unable to get start frame ({start_frame_index})");
                continue;
            };
            let Some(end_frame) = bvh.frames().nth(end_frame_index) else {
                error!("Unable to get end frame ({end_frame_index})");
                continue;
            };

            // Time distance between start frame and current trajectory's time stamp.
            let factor = time - start_frame_index as f32 * frame_time;
            let mut euler = Vec3::ZERO;
            let mut translation = Vec3::ZERO;

            for channel in root_joint.data().channels() {
                // SAFETY: We assume that the provided channel exists in the motion data.
                let start = *start_frame.get(channel).unwrap();
                let end = *end_frame.get(channel).unwrap();
                let data = f32::lerp(start, end, factor);
                match channel.channel_type() {
                    ChannelType::RotationX => euler.x = data.to_radians(),
                    ChannelType::RotationY => euler.y = data.to_radians(),
                    ChannelType::RotationZ => euler.z = data.to_radians(),
                    ChannelType::PositionX => translation.x = data,
                    ChannelType::PositionY => translation.y = data,
                    ChannelType::PositionZ => translation.z = data,
                }
            }

            self.matrices.push(Mat4::from_rotation_translation(
                Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z),
                translation,
            ));
        }
    }

    // TODO: Get time from chunk index, and chunk offset index
    /// Create an iterator that iterates through all trajectory matrices chunk by chunk.
    pub fn iter_chunk(&self) -> impl Iterator<Item = &[Mat4]> {
        let chunk_count = self.chunk_count();
        (0..chunk_count).map(|c| self.get_chunk(c))
    }

    /// Create an iterator that iterates through the trajectory matrices inside a given chunk index.
    pub fn get_chunk(&self, chunk_index: usize) -> &[Mat4] {
        let start_index = self.offsets[chunk_index];
        let end_index = self.offsets[chunk_index + 1];

        &self.matrices[start_index..end_index]
    }

    /// Number of trajectory matrices chunks.
    pub fn chunk_count(&self) -> usize {
        usize::max(self.offsets.len() - 1, 0)
    }

    /// Calculate the time value from a chunk offset index.
    /// This is best used alongside with [`iter_chunk`][Self::iter_chunk].
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_motion_matching::motion_data_asset::Trajectories;
    ///
    /// let trajectories = Trajectories::new(0.1667);
    /// // Append frames here...
    ///
    /// for chunk in trajectories.iter_chunk() {
    ///     for (chunk_offset, _) in chunk.enumerate() {
    ///         let time = trajectories.time_from_chunk_offset(chunk_offset);
    ///         println!("Time: {}", time);
    ///     }
    /// }
    /// ```
    pub fn time_from_chunk_offset(&self, chunk_offset: usize) -> f32 {
        chunk_offset as f32 * self.interval
    }

    /// Calculate the floored chunk offset index from a time value.
    pub fn chunk_offset_from_time(&self, time: f32) -> usize {
        (time / self.interval) as usize
    }
}

// Getter functions
impl Trajectories {
    pub fn matrices(&self) -> &[Mat4] {
        &self.matrices
    }

    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    pub fn interval(&self) -> f32 {
        self.interval
    }
}

/// Serializable joint with minimal required data.
#[derive(Serialize, Deserialize, Debug)]
pub struct JointInfo {
    /// Name of joint.
    name: String,
    /// Offset position of joint.
    offset: Vec3,
    /// Parent index of this joint.
    parent_index: Option<usize>,
    /// Information needed for referencing pose data.
    pose_refs: Vec<PoseRef>,
}

impl JointInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> Vec3 {
        self.offset
    }

    pub fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }

    pub fn pose_refs(&self) -> &[PoseRef] {
        &self.pose_refs
    }
}

impl JointInfo {
    pub fn from_joint_data(joint_data: &JointData) -> Self {
        Self {
            name: joint_data.name().to_string(),
            offset: Vec3::new(
                joint_data.offset().x,
                joint_data.offset().y,
                joint_data.offset().z,
            ),
            parent_index: joint_data.parent_index(),
            pose_refs: joint_data
                .channels()
                .iter()
                .map(|c| PoseRef::from(*c))
                .collect(),
        }
    }
}

impl JointTrait for JointInfo {
    fn channels(&self) -> impl Iterator<Item = impl JointChannelTrait> {
        self.pose_refs.iter()
    }

    fn offset(&self) -> Vec3 {
        self.offset
    }

    fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PoseRef {
    pose_index: usize,
    data_type: PoseDataType,
}

impl JointChannelTrait for &PoseRef {
    fn channel_type(&self) -> ChannelType {
        match self.data_type {
            PoseDataType::RotationX => ChannelType::RotationX,
            PoseDataType::RotationY => ChannelType::RotationY,
            PoseDataType::RotationZ => ChannelType::RotationZ,
            PoseDataType::PositionX => ChannelType::PositionX,
            PoseDataType::PositionY => ChannelType::PositionY,
            PoseDataType::PositionZ => ChannelType::PositionZ,
        }
    }

    fn motion_index(&self) -> usize {
        self.pose_index
    }
}

impl From<Channel> for PoseRef {
    fn from(value: Channel) -> Self {
        Self {
            pose_index: value.motion_index(),
            data_type: PoseDataType::from(value.channel_type()),
        }
    }
}

/// The available degrees of freedom along which a `Joint` may be manipulated.
///
/// A complete serializable match of [`ChannelType`].
#[derive(Serialize, Deserialize, Debug)]
pub enum PoseDataType {
    /// Can be rotated along the `x` axis.
    RotationX,
    /// Can be rotated along the `y` axis.
    RotationY,
    /// Can be rotated along the `z` axis.
    RotationZ,
    /// Can be translated along the `x` axis.
    PositionX,
    /// Can be translated along the `y` axis.
    PositionY,
    /// Can be translated along the `z` axis.
    PositionZ,
}

impl From<ChannelType> for PoseDataType {
    fn from(value: ChannelType) -> Self {
        match value {
            ChannelType::RotationX => Self::RotationX,
            ChannelType::RotationY => Self::RotationY,
            ChannelType::RotationZ => Self::RotationZ,
            ChannelType::PositionX => Self::PositionX,
            ChannelType::PositionY => Self::PositionY,
            ChannelType::PositionZ => Self::PositionZ,
        }
    }
}

#[derive(Default)]
struct MotionDataAssetLoader;

impl AssetLoader for MotionDataAssetLoader {
    type Asset = MotionDataAsset;
    type Settings = ();
    type Error = MotionDataLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let motion_data = serde_json::from_slice::<MotionDataAsset>(&bytes)?;

        Ok(motion_data)
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
