use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bvh_anim::Bvh;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub struct BvhAssetPlugin;

impl Plugin for BvhAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BvhAsset>()
            .init_asset_loader::<BvhAssetLoader>();
    }
}

#[derive(Asset, TypePath, Deref)]
pub struct BvhAsset {
    #[deref]
    bvh: Bvh,
    loopable: bool,
    name: String,
}

impl BvhAsset {
    pub fn loopable(&self) -> bool {
        self.loopable
    }

    pub fn name(&self) -> &String {
        &self.name
    }
}

#[derive(Default)]
pub struct BvhAssetLoader;

impl AssetLoader for BvhAssetLoader {
    type Asset = BvhAsset;
    type Settings = BvhAssetSettings;
    type Error = BvhAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let bvh = bvh_anim::from_bytes(bytes)?;
        Ok(BvhAsset {
            bvh,
            loopable: settings.loopable,
            name: load_context
                .path()
                .file_name()
                .unwrap_or_default()
                .to_owned()
                .to_string_lossy()
                .to_string(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bvh"]
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Copy)]
pub struct BvhAssetSettings {
    pub loopable: bool,
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
