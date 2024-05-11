use bevy::prelude::*;
use bvh_anim::{errors::LoadError, Bvh};
use std::{fs, io::BufReader};

pub struct AnimationLoaderPlugin;

impl Plugin for AnimationLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, store_bvh_file)
            .add_systems(Update, get_animation_data)
            .insert_resource(BvhData::default())
            .insert_resource(BvhFile(Vec::new()))
            .add_event::<AnimationSelectEvent>();
    }
}

const ANIMATION_FILE_PATH: &str = "./assets/walking-animation-dataset/";

#[derive(Resource, Default)]
pub struct BvhData {
    pub bvh_animation: Option<Bvh>,
}

#[derive(Resource)]
pub struct BvhFile(pub Vec<String>);

#[derive(Event)]
pub struct AnimationSelectEvent(pub String);

pub fn load_bvh_file() -> Result<Vec<String>, LoadError> {
    let mut bvh_file: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(ANIMATION_FILE_PATH) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    bvh_file.push(filename.to_string());
                }
            }
        }
    } else {
        println!("Failed to read directory");
    }

    Ok(bvh_file)
}

pub fn store_bvh_file(mut commands: Commands) {
    match load_bvh_file() {
        Ok(bvh_file) => commands.insert_resource(BvhFile(bvh_file)),
        Err(err) => {
            commands.insert_resource(BvhFile(Vec::new()));
            println!("{:#?}", err);
        }
    }
}

pub fn get_animation_data(
    mut commands: Commands,
    mut event_reader: EventReader<AnimationSelectEvent>,
) {
    for event in event_reader.read() {
        // println!("Default File: {}", default_file);
        println!("Event: {}", event.0);

        // default_file = &event.0;
        let content = ANIMATION_FILE_PATH.to_owned() + &event.0;

        // let content = ANIMATION_FILE_PATH.to_owned() + "walk1_subject1.bvh";
        let bvh_file: fs::File = fs::File::open(&content).unwrap();
        let bvh_reader: BufReader<fs::File> = BufReader::new(bvh_file);

        let bvh: Bvh = bvh_anim::from_reader(bvh_reader).unwrap();

        commands.insert_resource(BvhData {
            bvh_animation: Some(bvh),
        });
    }
}
