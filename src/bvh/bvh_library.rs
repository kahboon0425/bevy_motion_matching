use std::{path::PathBuf, str::FromStr};

use bevy::{prelude::*, utils::HashSet};

use crate::bvh_asset::{BvhAsset, BvhAssetPlugin};

pub const BVH_FOLDER: &str = "bvh";

pub struct BvhLibraryPlugin;

impl Plugin for BvhLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BvhAssetPlugin)
            .init_resource::<BvhLibrary>()
            .add_systems(Startup, load_bvh_library);
    }
}

fn load_bvh_library(mut bvh_library_manager: BvhLibraryManager, asset_server: Res<AssetServer>) {
    fn recursive_load_bvh(
        path: PathBuf,
        subpath: PathBuf,
        bvh_library_manager: &mut BvhLibraryManager,
        asset_server: &Res<AssetServer>,
    ) {
        let Ok(entries) = std::fs::read_dir(&path) else {
            error!("Failed to read Bvh directory: {:?}", path);
            return;
        };

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };

            let mut new_subpath = subpath.clone();
            new_subpath.push(entry.file_name());

            if entry.path().is_dir() {
                let mut new_path = path.clone();
                new_path.push(entry.file_name());
                recursive_load_bvh(new_path, new_subpath, bvh_library_manager, asset_server);
            } else if entry.path().is_file() {
                bvh_library_manager.load(asset_server, new_subpath);
            }
        }
    }

    // Relative path from the executable
    let mut path = PathBuf::from_str("./assets").unwrap();
    path.push(BVH_FOLDER);
    // Subpath starts from the BVH folder only
    let subpath = PathBuf::from_str(BVH_FOLDER).unwrap();

    recursive_load_bvh(path, subpath, &mut bvh_library_manager, &asset_server)

    // let Ok(entries) = std::fs::read_dir(String::from("assets/") + BVH_FOLDER) else {
    //     error!("Failed to read Bvh directory.");
    //     return;
    // };

    // for entry in entries {
    //     let Ok(entry) = entry else {
    //         return;
    //     };
    //     if let Some(filename) = entry.file_name().to_str() {
    //         bvh_library_manager.load(&asset_server, filename);
    //     }
    // }
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
    pub fn load(&mut self, asset_server: &AssetServer, file_path: PathBuf) {
        let handle = asset_server.load(file_path.clone());
        if self.bvh_library.library.insert(handle) == false {
            warn!("Same asset loaded again: {:?}", file_path);
        }
    }
}
