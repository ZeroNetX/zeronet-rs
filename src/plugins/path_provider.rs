use std::ops::DerefMut;

use log::error;

use self::path_provider::PathProvider;

use crate::{
    environment::{ENV, PATH_PROVIDER_PLUGINS},
    impl_plugin,
    plugins::Permission,
};

impl_plugin!(
    PathProviderPlugin,
    PathProvider,
    "assets/plugins/path_provider.wit",
    Permission::PathProvider
);

pub fn get_storage_path() -> String {
    let mut plugin = (*PATH_PROVIDER_PLUGINS).write().unwrap();
    let plugin = plugin.deref_mut().first_mut().unwrap();
    let path = plugin
        .provider
        .get_storage_path(&mut plugin.store, &(&*ENV).data_path.display().to_string());
    if path.is_err() {
        error!("Failed to get storage path");
    }
    path.unwrap()
}

pub fn get_file_path(block_id: &str) -> String {
    let mut plugin = (*PATH_PROVIDER_PLUGINS).write().unwrap();
    let plugin = plugin.deref_mut().first_mut().unwrap();
    let path = plugin.provider.get_file_path(
        &mut plugin.store,
        &(&*ENV).data_path.display().to_string(),
        block_id,
    );
    if path.is_err() {
        error!("Failed to get storage path");
    }
    path.unwrap()
}
