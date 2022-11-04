mod peer_db;

mod auth_wrapper;
pub mod package;
pub mod path_provider;
pub mod site_server;
mod websocket;

use std::path::PathBuf;

use futures::executor::block_on;
use log::error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wit_bindgen_wasmer::wasmer::{imports, Cranelift, Module, Store};

use self::{manifest::Manifest, package::PluginManifest};

wit_bindgen_wasmer::import!("assets/plugins/manifest.wit");

use site_server::server::AppEntryImpl;

pub fn register_plugins<T: AppEntryImpl>(app: actix_web::App<T>) -> actix_web::App<T> {
    use actix_web::web::{get, scope};
    app.service(scope("/Authenticate").route("", get().to(auth_wrapper::serve_auth_wrapper_key)))
}

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
            let plugins = (&*$crate::environment::PLUGINS)
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

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    name: String,
    description: String,
    version: String,
    revision: i64,
    permissions: Vec<Permission>,
    #[serde(skip)]
    path: PathBuf,
}

impl Default for Plugin {
    fn default() -> Self {
        Plugin {
            version: "0.0.1".into(),
            revision: 1,
            name: Default::default(),
            description: Default::default(),
            path: Default::default(),
            permissions: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
enum Permission {
    PathProvider(String),
    #[default]
    None,
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D>(deserializer: D) -> Result<Permission, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Ok(Permission::from(s.as_str()))
    }
}

impl Serialize for Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = match self {
            Permission::PathProvider(version) => format!("path_provider@{}", version),
            Permission::None => "".into(),
        };
        serializer.serialize_str(&string)
    }
}

impl From<&str> for Permission {
    fn from(s: &str) -> Self {
        let mut splited = s.split('@');
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
