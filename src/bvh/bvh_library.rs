use std::{fs, path::PathBuf, str::FromStr};

use bevy::{prelude::*, utils::HashSet};
use bevy_bvh_anim::prelude::*;

pub const BVH_FOLDER: &str = "bvh";
pub const BVH_MAP_FOLDER: &str = "bvh_map";

pub struct BvhLibraryPlugin;

impl Plugin for BvhLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BvhAssetPlugin)
            .init_resource::<BvhLibrary>()
            .add_systems(Startup, load_bvh_library);
    }
}

/// Stores the [`Handle::Strong`] of all loaded Bvh assets.
#[derive(Resource, Default)]
pub struct BvhLibrary {
    map: Option<Handle<BvhAsset>>,
    library: HashSet<Handle<BvhAsset>>,
}

impl BvhLibrary {
    pub fn get_map(&self) -> Option<&Handle<BvhAsset>> {
        self.map.as_ref()
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct BvhLibraryManager<'w> {
    bvh_library: ResMut<'w, BvhLibrary>,
}

impl<'w> BvhLibraryManager<'w> {
    /// Loads Bvh data from disk.
    /// # Warning
    /// A warning will be issued if specified asset has been loaded before.
    pub fn load(&mut self, asset_server: &AssetServer, file_path: PathBuf) {
        let handle = asset_server.load(file_path.clone());
        if self.bvh_library.library.insert(handle) == false {
            warn!("Same Bvh asset loaded again: {:?}", file_path);
        }
    }

    /// Loads Bvh map data from disk.
    /// # Warning
    /// A warning will be issued if specified asset has been loaded before.
    pub fn load_map(&mut self, asset_server: &AssetServer, file_path: PathBuf) {
        if self.bvh_library.map.is_some() {
            warn!("Same Bvh map asset loaded again: {:?}", file_path);
        }
        let handle = asset_server.load(file_path);
        self.bvh_library.map = Some(handle);
    }
}

/// Load all bvh data from [bvh folder](BVH_FOLDER) and bvh map from [bvh map folder](BVH_MAP_FOLDER).
fn load_bvh_library(mut bvh_library_manager: BvhLibraryManager, asset_server: Res<AssetServer>) {
    /// Recursively load bvh data from [bvh folder](BVH_FOLDER).
    fn recursive_load_bvh(
        path: PathBuf,
        subpath: PathBuf,
        bvh_library_manager: &mut BvhLibraryManager,
        asset_server: &Res<AssetServer>,
    ) {
        let Ok(entries) = fs::read_dir(&path) else {
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

    let asset_path = PathBuf::from_str("./assets").unwrap();
    // Relative path from the executable
    let mut path = asset_path.clone();
    path.push(BVH_FOLDER);
    // Subpath starts from the bvh folder only
    let subpath = PathBuf::from_str(BVH_FOLDER).unwrap();

    recursive_load_bvh(path, subpath, &mut bvh_library_manager, &asset_server);

    // Loads the bvh map asset
    // Relative path from the executable
    let mut path = asset_path.clone();
    path.push(BVH_MAP_FOLDER);
    // Subpath starts from the bvh map folder only
    let mut subpath = PathBuf::from_str(BVH_MAP_FOLDER).unwrap();

    let Ok(mut entries) = fs::read_dir(&path) else {
        error!("Unable to read Bvh Map directory: {:?}", path);
        return;
    };

    if let Some(entry) = entries.next().and_then(|e| e.ok()) {
        if entry.path().is_file() {
            subpath.push(entry.file_name());
            bvh_library_manager.load_map(&asset_server, subpath);
        } else {
            warn!("Only files are supported in the `{BVH_MAP_FOLDER}` folder.")
        }
    }

    if entries.next().is_some() {
        warn!("More than 1 entries detected in `{BVH_MAP_FOLDER}` folder, only the first one is loaded.");
    }
}
