use actix::Actor;
use actix_web::{
    web::{get, scope, Data},
    App,
};

use crate::controllers::server::AppEntryImpl;

use self::{
    auth_wrapper::serve_auth_wrapper_key,
    websocket::{events::WebsocketController, serve_websocket},
};

mod auth_wrapper;
mod websocket;

pub use websocket::SiteAnnounce;

pub fn register_plugins<T: AppEntryImpl>(app: App<T>) -> App<T> {
    app.service(scope("/Authenticate").route("", get().to(serve_auth_wrapper_key)))
}

pub fn register_site_plugins<T: AppEntryImpl>(app: App<T>) -> App<T> {
    let websocket_controller = WebsocketController { listeners: vec![] }.start();
    app.app_data(Data::new(websocket_controller))
        .service(scope("/ZeroNet-Internal").route("/Websocket", get().to(serve_websocket)))
}
