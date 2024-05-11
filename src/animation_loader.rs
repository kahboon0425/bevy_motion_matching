use bevy::{asset::Handle, prelude::*};
use bvh_anim::{errors::LoadError, Bvh};
use std::{fs, io::BufReader};

pub struct AnimationLoaderPlugin;

impl Plugin for AnimationLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<BvhAsset>()
            .insert_resource(BvhHandles::default())
            .add_systems(Startup, store_bvh);
    }
}

#[derive(Resource, Clone, Default)]
pub struct BvhHandles {
    pub handles: Vec<Handle<BvhAsset>>,
}

#[derive(Asset, TypePath, Clone, Default)]
pub struct BvhAsset {
    pub asset: Bvh,
}

pub fn load_bvh() -> Result<Vec<Bvh>, LoadError> {
    let animation_file_path: &str = "./assets/walking-animation-dataset/";

    let mut loaded_bvhs: Vec<Bvh> = Vec::new();

    if let Ok(entries) = fs::read_dir(animation_file_path) {
        // We only load to entires for now
        for entry in entries.take(2) {
            let Ok(entry) = entry else {
                continue;
            };

            let filename = entry.file_name();
            let Some(filename) = filename.to_str() else {
                continue;
            };

            let filename: String = animation_file_path.to_owned() + filename;

            let bvh_file = fs::File::open(&filename).unwrap();
            let bvh_reader = BufReader::new(bvh_file);

            let bvh: Bvh = bvh_anim::from_reader(bvh_reader)?;

            loaded_bvhs.push(bvh);
        }

        if loaded_bvhs.is_empty() {
            println!("No BVH files found");
        }
    } else {
        println!("Failed to read directory");
    }

    Ok(loaded_bvhs)
}

pub fn store_bvh(mut assets: ResMut<Assets<BvhAsset>>, mut bvh_handles: ResMut<BvhHandles>) {
    match load_bvh() {
        Ok(bvhs) => {
            for bvh in bvhs {
                // Create a new BvhAsset and insert it into the asset server
                bvh_handles
                    .handles
                    .push(assets.add(BvhAsset { asset: bvh.into() }));
            }
        }
        Err(err) => {
            println!("{:#?}", err);
        }
    }
}
