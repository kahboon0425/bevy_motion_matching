use bevy::prelude::*;
use bvh_anim::{Channel, ChannelType, JointData};

/// Trait functions required to get joint channel data.
pub trait JointChannelTrait {
    fn channel_type(&self) -> ChannelType;
    fn motion_index(&self) -> usize;
}

/// Trait functions required to get joint data.
pub trait JointTrait {
    fn channels(&self) -> impl Iterator<Item = impl JointChannelTrait>;
    fn offset(&self) -> Vec3;
    fn parent_index(&self) -> Option<usize>;
}

impl JointTrait for JointData {
    fn channels(&self) -> impl Iterator<Item = impl JointChannelTrait> {
        self.channels().iter()
    }

    fn offset(&self) -> Vec3 {
        let offset = JointData::offset(self);
        Vec3::new(offset.x, offset.y, offset.z)
    }

    fn parent_index(&self) -> Option<usize> {
        JointData::parent_index(self)
    }
}

impl JointChannelTrait for &Channel {
    fn channel_type(&self) -> ChannelType {
        Channel::channel_type(self)
    }

    fn motion_index(&self) -> usize {
        Channel::motion_index(self)
    }
}
