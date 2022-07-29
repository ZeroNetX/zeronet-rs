use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web::{get, scope},
    App,
};

use self::{auth_wrapper::serve_auth_wrapper_key, websocket::serve_websocket};

mod auth_wrapper;
mod websocket;

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
    app.service(scope("/ZeroNet-Internal").route("/Websocket", get().to(serve_websocket)))
}
