use std::{collections::HashMap, str::FromStr};

use actix_web::{
    web::{Data, Query},
    HttpRequest, HttpResponse,
};
use futures::executor::block_on;
use log::*;
use uuid::Uuid;

use crate::{
    core::address::Address,
    environment::ENV,
    plugins::site_server::{handlers::sites::AddWrapperKey, server::ZeroServer},
};

pub async fn serve_auth_wrapper_key(
    req: HttpRequest,
    query: Query<HashMap<String, String>>,
) -> HttpResponse {
    let nonce = Uuid::new_v4().simple().to_string();
    let data = req.app_data::<Data<ZeroServer>>().unwrap();
    {
        let mut nonces = data.wrapper_nonces.lock().unwrap();
        nonces.insert(nonce.clone());
        trace!("Valid nonces ({}): {:?}", nonces.len(), nonces);
    }
    let def = String::default();
    let address_string = query.get("address").unwrap_or(&def);
    let address = match Address::from_str(address_string) {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::Ok()
                .body(format!("{} is a malformed ZeroNet address", address_string));
        }
    };
    let access_key = query.get("access_key").unwrap_or(&def);
    match access_key.as_str() {
        "" => return HttpResponse::Ok().body(
            "This API is restricted, use access_key param to Authenticate, get valid wrapper key",
        ),
        key => {
            if key != &*ENV.access_key {
                return HttpResponse::Ok().body("Provided access_key is not Valid");
            }
        }
    }
    trace!("Serving wrapper key for {}", address);
    let result = data
        .site_controller
        .send(AddWrapperKey::new(address, nonce.clone()));
    let result = block_on(result);

    match result {
        Ok(_) => match result {
            Ok(_) => HttpResponse::Ok().body(format!("wrapper_key={}", nonce)),
            Err(err) => {
                error!("Bad request {}", err);
                HttpResponse::BadRequest().finish()
            }
        },
        Err(err) => {
            error!("Error sending wrapper key to site manager");
            error!("Bad request {}", err);
            HttpResponse::BadRequest().finish()
        }
    }
}
