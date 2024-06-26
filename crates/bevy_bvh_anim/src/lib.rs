pub use bvh_anim;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::{
        thiserror::{self, Error},
        BoxedFuture,
    },
};
use bvh_anim::Bvh;

pub mod prelude {
    pub use crate::{BvhAsset, BvhAssetPlugin};
    pub use bvh_anim::{
        bvh, Axis as BvhAxis, Bvh, Channel, Frame, Frames, Joint, JointData, JointName,
    };
}

pub struct BvhAssetPlugin;

impl Plugin for BvhAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BvhAsset>()
            .init_asset_loader::<BvhAssetLoader>();
    }
}

#[derive(Asset, TypePath)]
pub struct BvhAsset(Bvh);

impl BvhAsset {
    pub fn get(&self) -> &Bvh {
        &self.0
    }
}

#[derive(Default)]
pub struct BvhAssetLoader;

impl AssetLoader for BvhAssetLoader {
    type Asset = BvhAsset;
    type Settings = ();
    type Error = BvhAssetLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let bvh = bvh_anim::from_bytes(bytes)?;
            Ok(BvhAsset(bvh))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bvh"]
    }
}

/// Possible errors that can be produced by [`BvhAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum BvhAssetLoaderError {
    /// An [Io](std::io) Error
    #[error("Could not load bvh file: {0}")]
    Io(#[from] std::io::Error),
    /// A [Bvh](bvh_anim::errors::LoadError) Error
    #[error("Could not load bvh: {0}")]
    BvhLoadError(#[from] bvh_anim::errors::LoadError),
}
