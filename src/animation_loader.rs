use bevy::{
    asset::{Asset, Handle},
    ecs::system::SystemParamItem,
    prelude::*,
    render::render_asset::{self, RenderAsset, RenderAssetPlugin, RenderAssetUsages},
};
use bvh_anim::{errors::LoadError, Bvh};
use std::{fs, io::BufReader};

pub struct AnimationLoaderPlugin;

impl Plugin for AnimationLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<BvhAsset>::default())
            .init_asset::<BvhAsset>()
            .add_systems(Startup, store_bvh);
    }
}

#[derive(Component, Clone, Default)]
pub struct BvhHandle {
    pub bvh_assets: Handle<BvhAsset>,
}

#[derive(Asset, TypePath, Clone, Default)]
pub struct BvhAsset {
    pub asset: Bvh,
}

impl RenderAsset for BvhAsset {
    type PreparedAsset = Self;

    type Param = ();

    fn asset_usage(&self) -> RenderAssetUsages {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        self,
        _param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, render_asset::PrepareAssetError<Self>> {
        Ok(self)
    }
}

pub fn load_bvh() -> Result<Vec<Bvh>, LoadError> {
    let animation_file_path: &str = "./assets/walking-animation-dataset/";

    let mut loaded_bvhs: Vec<Bvh> = Vec::new();

    let mut count: usize = 0;
    if let Ok(entries) = fs::read_dir(animation_file_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    let filename: String = animation_file_path.to_owned() + filename;

                    let bvh_file: fs::File = fs::File::open(&filename).unwrap();
                    let bvh_reader: BufReader<fs::File> = BufReader::new(bvh_file);

                    let bvh: Bvh = bvh_anim::from_reader(bvh_reader)?;

                    loaded_bvhs.push(bvh);

                    if count >= 2 {
                        break;
                    }
                    count += 1;
                }
            }
        }

        if loaded_bvhs.is_empty() {
            println!("No BVH files found");
        }
    } else {
        println!("Failed to read directory");
    }

    Ok(loaded_bvhs)
}

pub fn store_bvh(mut commands: Commands, mut assets: ResMut<Assets<BvhAsset>>) {
    match load_bvh() {
        Ok(bvhs) => {
            for bvh in bvhs {
                // Create a new BvhAsset and insert it into the asset server
                commands.spawn(BvhHandle {
                    bvh_assets: assets.add(BvhAsset { asset: bvh.into() }),
                    ..default()
                });
            }
        }
        Err(err) => {
            println!("{:#?}", err);
        }
    }
}
