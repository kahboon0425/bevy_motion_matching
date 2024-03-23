use bevy::prelude::*;
use bvh_anim::{errors::LoadError, Bvh};
use std::{fs, io::BufReader};

#[derive(Resource)]
pub struct BvhData {
    pub bvh_animation: Option<Vec<Bvh>>,
    pub current_frame_index: usize,
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
                bvh_animation: Some(bvhs),
                current_frame_index: 0,
            });
        }
        Err(err) => {
            commands.insert_resource(BvhData {
                bvh_animation: None,
                current_frame_index: 0,
            });
            println!("{:#?}", err);
        }
    }
}
