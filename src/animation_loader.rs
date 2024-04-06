use bevy::prelude::*;
use bvh_anim::{errors::LoadError, Bvh, Frames};
use std::{fs, io::BufReader};

pub struct AnimationLoaderPlugin;

impl Plugin for AnimationLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, store_bvh);
        app.insert_resource(BvhData::new());
    }
}

#[derive(Resource)]
pub struct BvhData {
    pub bvh_animation: Vec<Bvh>,
}

impl BvhData {
    pub fn new() -> Self {
        Self {
            bvh_animation: Vec::new(),
        }
    }

    pub fn get_bvh_animation_data(&self, animation_data_index: usize) -> &Bvh {
        return &self.bvh_animation[animation_data_index];
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
                    // println!("Loading animation file: {}", filename);

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

pub fn store_bvh(mut commands: Commands) {
    match load_bvh() {
        Ok(bvhs) => {
            commands.insert_resource(BvhData {
                bvh_animation: bvhs,
            });
        }
        Err(err) => {
            commands.insert_resource(BvhData {
                bvh_animation: Vec::new(),
            });
            println!("{:#?}", err);
        }
    }
}
