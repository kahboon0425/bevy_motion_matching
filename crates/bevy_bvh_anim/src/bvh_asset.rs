use std::convert::Infallible;

use bevy::asset::io::Reader;
use bevy::asset::transformer::{AssetTransformer, TransformedAsset};
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
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

#[derive(Asset, TypePath)]
pub struct BvhAsset {
    bvh: Bvh,
    loopable: bool,
}

impl BvhAsset {
    pub fn get(&self) -> &Bvh {
        &self.bvh
    }
}

#[derive(Default)]
pub struct BvhAssetLoader;

impl AssetLoader for BvhAssetLoader {
    type Asset = BvhAsset;
    type Settings = ();
    type Error = BvhAssetLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let bvh = bvh_anim::from_bytes(bytes)?;
        Ok(BvhAsset {
            bvh,
            loopable: false,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bvh"]
    }
}

#[derive(Default)]
pub struct BvhAssetTransformer;

#[derive(Default, Serialize, Deserialize)]
pub struct BvhAssetSettings {
    pub loopable: bool,
}

impl AssetTransformer for BvhAssetTransformer {
    type AssetInput = BvhAsset;
    type AssetOutput = BvhAsset;
    type Settings = BvhAssetSettings;
    type Error = Infallible;

    async fn transform<'a>(
        &'a self,
        mut asset: TransformedAsset<Self::AssetInput>,
        settings: &'a Self::Settings,
    ) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
        asset.loopable = settings.loopable;
        Ok(asset)
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
