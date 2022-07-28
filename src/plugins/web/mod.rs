use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web::{get, scope},
    App,
};

use self::auth_wrapper::serve_auth_wrapper_key;

mod auth_wrapper;

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
