use bevy::{
    prelude::*,
    utils::{hashbrown::hash_map::Keys, HashMap},
};

use crate::bvh_asset::{BvhAsset, BvhAssetPlugin};

pub const BVH_FOLDER: &str = "bvh/";

pub struct BvhLibraryPlugin;

impl Plugin for BvhLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BvhAssetPlugin)
            .init_resource::<BvhLibrary>()
            .add_systems(Startup, load_bvh_library);
    }
}

fn load_bvh_library(mut bvh_library_manager: BvhLibraryManager, asset_server: Res<AssetServer>) {
    let Ok(entries) = std::fs::read_dir(String::from("assets/") + BVH_FOLDER) else {
        error!("Failed to read Bvh directory.");
        return;
    };

    for entry in entries {
        let Ok(entry) = entry else {
            return;
        };
        if let Some(filename) = entry.file_name().to_str() {
            bvh_library_manager.load(&asset_server, filename)
        }
    }
}

#[derive(Resource, Default)]
pub struct BvhLibrary {
    library: HashMap<String, Handle<BvhAsset>>,
}

impl BvhLibrary {
    pub fn get_filenames(&self) -> Keys<'_, String, Handle<BvhAsset>> {
        self.library.keys()
    }

    pub fn get_handle(&self, filename: &str) -> Option<Handle<BvhAsset>> {
        self.library.get(filename).cloned()
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct BvhLibraryManager<'w> {
    bvh_library: ResMut<'w, BvhLibrary>,
}

impl<'w> BvhLibraryManager<'w> {
    /// Loads Bvh data from disk.
    /// Filename provided must be located inside "assets/bvh/".
    pub fn load(&mut self, asset_server: &AssetServer, filename: &str) {
        let handle = asset_server.load(BVH_FOLDER.to_owned() + filename);
        // Add or replace handle.
        self.bvh_library.library.insert(filename.to_owned(), handle);
    }
}
