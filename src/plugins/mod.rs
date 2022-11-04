pub mod core;
pub mod utils;

pub mod path_provider;
pub mod site_server;

mod auth_wrapper;
mod peer_db;
mod websocket;

wit_bindgen_wasmer::import!("assets/plugins/manifest.wit");

use site_server::server::AppEntryImpl;

pub fn register_plugins<T: AppEntryImpl>(app: actix_web::App<T>) -> actix_web::App<T> {
    use actix_web::web::{get, scope};
    app.service(scope("/Authenticate").route("", get().to(auth_wrapper::serve_auth_wrapper_key)))
}
