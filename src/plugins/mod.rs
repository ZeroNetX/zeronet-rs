mod peer_db;

mod auth_wrapper;
pub mod core;
pub mod path_provider;
pub mod site_server;
mod websocket;

use std::path::PathBuf;

use futures::executor::block_on;
use log::error;
use wit_bindgen_wasmer::wasmer::{imports, Cranelift, Module, Store};

use self::{
    core::{manifest::PluginManifest, plugin::Plugin},
    manifest::Manifest,
};

wit_bindgen_wasmer::import!("assets/plugins/manifest.wit");

use site_server::server::AppEntryImpl;

pub fn register_plugins<T: AppEntryImpl>(app: actix_web::App<T>) -> actix_web::App<T> {
    use actix_web::web::{get, scope};
    app.service(scope("/Authenticate").route("", get().to(auth_wrapper::serve_auth_wrapper_key)))
}

pub fn load_plugins() -> Vec<Plugin> {
    let engine = Cranelift::default();
    let mut store = Store::new(engine);
    let plugins_dir = PathBuf::from("plugins");
    if plugins_dir.exists() {
        let list = std::fs::read_dir(plugins_dir).unwrap();
        let plugins = list.filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_str().unwrap();
                let plugin_dir = path.display().to_string();
                let manifest = PathBuf::from(format!("{}/manifest.json", plugin_dir));
                let plugin = PathBuf::from(format!("{}/{}.wasm", plugin_dir, name));
                let has_manifest = manifest.is_file();
                let has_plugin = plugin.is_file();
                if has_manifest && has_plugin {
                    let res = block_on(PluginManifest::load(name));
                    if let Ok(manifest) = res {
                        let verified = block_on(manifest.verify_plugin()).unwrap();
                        if verified {
                            return Some(plugin);
                        }
                    }
                }
            }
            None
        });
        let mut plugins_loaded = Vec::new();
        for plugin in plugins {
            let bytes = std::fs::read(&plugin).unwrap();
            let module = Module::new(&store, bytes);
            if let Ok(module) = module {
                let mut imports = imports! {};
                let funs = Manifest::instantiate(&mut store, &module, &mut imports);
                if let Ok((manifest, _)) = funs {
                    let name = manifest.name(&mut store).unwrap();
                    let description = manifest.description(&mut store).unwrap();
                    let version = manifest.version(&mut store).unwrap();
                    let revision = manifest.revision(&mut store).unwrap();
                    let permissions = manifest
                        .permissions(&mut store)
                        .unwrap()
                        .into_iter()
                        .map(|s| s.as_str().into())
                        .collect();
                    let path = plugin.clone();
                    let plugin = Plugin {
                        name,
                        description,
                        version,
                        revision,
                        permissions,
                        path,
                    };
                    plugins_loaded.push(plugin);
                } else {
                    let error = funs.err().unwrap();
                    error!("Failed to load plugin {:?}", error);
                }
            }
        }
        plugins_loaded
    } else {
        vec![]
    }
}
