use bevy::{prelude::*, utils::HashSet};

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
            bvh_library_manager.load(&asset_server, filename);
        }
    }
}

/// Stores the [`Handle::Strong`] of all loaded Bvh assets.
#[derive(Resource, Default)]
pub struct BvhLibrary {
    library: HashSet<Handle<BvhAsset>>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct BvhLibraryManager<'w> {
    bvh_library: ResMut<'w, BvhLibrary>,
}

impl<'w> BvhLibraryManager<'w> {
    /// Loads Bvh data from disk.
    /// Filename provided must be located inside "assets/bvh/".
    /// If specified asset has been loaded before, a warning will be issued.
    pub fn load(&mut self, asset_server: &AssetServer, filename: &str) {
        let handle = asset_server.load(BVH_FOLDER.to_owned() + filename);
        if self.bvh_library.library.insert(handle) == false {
            warn!("Same asset loaded again: {}", filename);
        }
    }
}
