use bevy::prelude::*;
use bvh_anim::Bvh;

pub struct AnimationLoaderPlugin;

impl Plugin for AnimationLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, store_bvh_file)
            .insert_resource(BvhData::default())
            .insert_resource(BvhFiles::default());
    }
}

pub fn store_bvh_file(mut bvh_files: ResMut<BvhFiles>) {
    let Ok(entries) = std::fs::read_dir(BVH_FOLDER) else {
        error!("Failed to read Bvh directory.");
        return;
    };

    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(filename) = entry.file_name().to_str() {
                bvh_files.file_names.push(filename.to_string());
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct BvhData {
    pub bvh_animation: Option<Bvh>,
}

#[derive(Resource, Default)]
pub struct BvhFiles {
    pub file_names: Vec<String>,
}
