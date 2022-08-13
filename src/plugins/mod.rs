mod path_provider;
mod peer_db;

pub mod web;

use std::path::PathBuf;

use log::error;
pub use path_provider::*;

use wit_bindgen_wasmer::wasmer::{imports, Cranelift, Module, Store};

use self::manifest::Manifest;

wit_bindgen_wasmer::import!("assets/plugins/manifest.wit");

pub fn load_plugins() -> Vec<Plugin> {
    let engine = Cranelift::default();
    let mut store = Store::new(engine);
    let plugins_dir = PathBuf::from("plugins");
    if plugins_dir.exists() {
        let list = std::fs::read_dir(plugins_dir).unwrap();
        let plugins = list.filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().unwrap() == "wasm" {
                Some(path)
            } else {
                None
            }
        });
        let mut plugins_loaded = Vec::new();
        for plugin in plugins {
            let bytes = std::fs::read(&plugin).unwrap();
            let module = Module::new(&store, &bytes);
            if let Ok(module) = module {
                let mut imports = imports! {};
                let funs = Manifest::instantiate(&mut store, &module, &mut imports);
                if let Ok((manifest, _)) = funs {
                    let name = manifest.name(&mut store).unwrap();
                    let description = manifest.description(&mut store).unwrap();
                    let version = manifest.version(&mut store).unwrap();
                    let permissions = manifest
                        .permissions(&mut store)
                        .unwrap()
                        .into_iter()
                        .map(|s| match s.as_str() {
                            "path_provider" => Permission::PathProvider,
                            _ => unimplemented!(),
                        })
                        .collect();
                    let path = plugin.clone();
                    let plugin = Plugin {
                        name,
                        description,
                        version,
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct Plugin {
    name: String,
    description: String,
    version: i64,
    permissions: Vec<Permission>,
    path: PathBuf,
}

#[derive(Debug, PartialEq)]
enum Permission {
    PathProvider,
}
