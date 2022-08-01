use actix::Actor;
use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web::{get, scope, Data},
    App,
};

use self::{
    auth_wrapper::serve_auth_wrapper_key,
    websocket::{events::WebsocketController, serve_websocket},
};

mod auth_wrapper;
mod websocket;

pub use websocket::SiteAnnounce;

pub fn register_plugins<
    T: ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
>(
    app: App<T>,
) -> App<T> {
    app.service(scope("/Authenticate").route("", get().to(serve_auth_wrapper_key)))
}

pub fn register_site_plugins<
    T: ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
>(
    app: App<T>,
) -> App<T> {
    let websocket_controller = WebsocketController { listeners: vec![] }.start();
    app.app_data(Data::new(websocket_controller))
        .service(scope("/ZeroNet-Internal").route("/Websocket", get().to(serve_websocket)))
}
