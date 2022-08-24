mod peer_db;

pub mod path_provider;
pub mod web;

use std::path::PathBuf;

use log::error;

use wit_bindgen_wasmer::wasmer::{imports, Cranelift, Module, Store};

use self::manifest::Manifest;

wit_bindgen_wasmer::import!("assets/plugins/manifest.wit");

#[macro_export]
macro_rules! impl_plugin {
    ($name:ident, $provider:ident, $plugin:expr, $permissions:expr) => {
        wit_bindgen_wasmer::import!($plugin);
        pub struct $name {
            pub store: wit_bindgen_wasmer::wasmer::Store,
            pub module: wit_bindgen_wasmer::wasmer::Module,
            pub provider: $provider,
        }

        pub fn load_plugins() -> mut_static::MutStatic<Vec<$name>> {
            let plugins = (&*crate::environment::PLUGINS)
                .iter()
                .filter(|p| p.permissions.contains(&$permissions));
            let mut plugins_loaded = Vec::new();
            for plugin in plugins {
                let engine = wit_bindgen_wasmer::wasmer::Cranelift::default();
                let mut store = wit_bindgen_wasmer::wasmer::Store::new(engine);
                let bytes = std::fs::read(&plugin.path).unwrap();
                let module = wit_bindgen_wasmer::wasmer::Module::new(&store, &bytes);
                if let Ok(module) = module {
                    let mut imports = wit_bindgen_wasmer::wasmer::imports! {};
                    let funs = $provider::instantiate(&mut store, &module, &mut imports);
                    if let Ok((provider, _)) = funs {
                        let plugin = $name {
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
            mut_static::MutStatic::from(plugins_loaded)
        }
    };
}

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

#[allow(dead_code)]
#[derive(Debug)]
pub struct Plugin {
    name: String,
    description: String,
    version: String,
    revision: i64,
    permissions: Vec<Permission>,
    path: PathBuf,
}

#[derive(Debug, PartialEq)]
enum Permission {
    PathProvider(String),
    None,
}

impl From<&str> for Permission {
    fn from(s: &str) -> Self {
        let mut splited = s.split("@").into_iter();
        let s = splited.next().unwrap();
        let version = splited.next().unwrap_or("0.0.1");
        match s {
            "path_provider" => Permission::PathProvider(version.into()),
            _ => Permission::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_from_str() {
        let permission = "path_provider";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::PathProvider("0.0.1".into()));

        let permission = "path_provider@0.0.2";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::PathProvider("0.0.2".into()));

        let permission = "path_provider#0.0.2";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::None);

        let permission = "";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::None);
    }
}
