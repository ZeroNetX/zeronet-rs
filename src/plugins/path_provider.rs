use std::ops::DerefMut;

use mut_static::MutStatic;
use wit_bindgen_wasmer::wasmer::{imports, Cranelift, Module, Store};

use crate::environment::{ENV, PATH_PROVIDER_PLUGINS, PLUGINS};

use self::path_provider::PathProvider;

use super::Permission;
use log::error;

wit_bindgen_wasmer::import!("assets/plugins/path_provider.wit");

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

pub fn load_path_provider_plugins() -> MutStatic<Vec<PathProviderPlugin>> {
    // let plugins_dir = PathBuf::from("plugins");
    let plugins = (&*PLUGINS)
        .iter()
        .filter(|p| p.permissions.contains(&Permission::PathProvider));
    let mut plugins_loaded = Vec::new();
    for plugin in plugins {
        let engine = Cranelift::default();
        let mut store = Store::new(engine);
        let bytes = std::fs::read(&plugin.path).unwrap();
        let module = Module::new(&store, &bytes);
        if let Ok(module) = module {
            let mut imports = imports! {};
            let funs = PathProvider::instantiate(&mut store, &module, &mut imports);
            if let Ok((provider, _)) = funs {
                let plugin = PathProviderPlugin {
                    store,
                    module,
                    provider,
                };
                plugins_loaded.push(plugin);
            } else {
                let error = funs.err().unwrap();
                error!("Failed to load plugin {:?}", error);
            }
        }
    }
    MutStatic::from(plugins_loaded)
}

#[allow(unused)]
pub struct PathProviderPlugin {
    store: Store,
    module: Module,
    provider: PathProvider,
}
