use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::prelude::Bvh;
use serde::{Deserialize, Serialize};

use super::chunk::{ChunkIterator, ChunkOffsets};

/// Stores chunks of trajectory matrices.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrajectoryData {
    /// Trajectory matrices.
    points: Vec<TrajectoryDataPoint>,
    /// Offset index of [`Self::matrices`] chunks.
    ///
    /// # Example
    ///
    /// \[0, 3, 5, 7\] contains chunk [0, 3), [3, 5), [5, 7)
    offsets: ChunkOffsets,
    /// Duration between each trajectory matrix in seconds.
    config: TrajectoryDataConfig,
}

impl TrajectoryData {
    pub fn new(config: TrajectoryDataConfig) -> Self {
        assert!(
            config.interval_time > 0.0,
            "Interval time between trajectories must be greater than 0!"
        );

        Self {
            points: Vec::new(),
            offsets: ChunkOffsets::new(),
            config,
        }
    }

    /// Append the trajectory points as a chunk while emptying the parsed in data points.
    pub(super) fn append_trajectory_chunk(&mut self, trajectory: &mut Vec<TrajectoryDataPoint>) {
        assert!(
            trajectory.len() >= self.config.point_len,
            "A trajectory must have at least the configured length: >={}",
            self.config.point_len
        );

        self.points.append(trajectory);
        self.offsets.push_chunk(trajectory.len());
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
        chunk_offset as f32 * self.config.interval_time
    }

    /// Calculate the floored chunk offset index from a time value.
    pub fn chunk_offset_from_time(&self, time: f32) -> usize {
        (time / self.config.interval_time) as usize
    }
}

impl ChunkIterator for TrajectoryData {
    type Item = TrajectoryDataPoint;

    fn offsets(&self) -> &ChunkOffsets {
        &self.offsets
    }

    fn items(&self) -> &[Self::Item] {
        &self.points
    }
}

// Getter functions
impl TrajectoryData {
    pub fn config(&self) -> &TrajectoryDataConfig {
        &self.config
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct TrajectoryDataPoint {
    pub matrix: Mat4,
    pub velocity: Vec2,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct TrajectoryDataConfig {
    /// Interval time between each data point.
    pub interval_time: f32,
    /// Number of data points per trajectory.
    pub point_len: usize,
}
