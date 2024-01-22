use actix::AsyncContext;
use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    super::{error::Error, request::Command, response::Message, ZeruWebsocket},
    users::{get_current_user, handle_cert_set},
};
use crate::{
    core::site::models::SiteInfo,
    environment::SITE_PERMISSIONS_DETAILS,
    plugins::site_server::handlers::{
        sites::{DBQueryRequest, SiteInfoListRequest, SiteInfoRequest},
        users::{UserCertAddRequest, UserCertDeleteRequest, UserSetSiteCertRequest, UserSiteData},
    },
    plugins::{
        site_server::handlers::{
            sites::{
                SiteBadFilesRequest, SiteDeleteRequest, SitePauseRequest, SitePermissionAddRequest,
                SitePermissionRemoveRequest, SiteResumeRequest, SiteSetSettingsValueRequest,
            },
            users::UserSiteDataDeleteRequest,
        },
        websocket::events::RegisterChannels,
    },
};

pub fn handle_cert_add(ws: &mut ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    let mut msg: UserCertAddRequest = serde_json::from_value(command.params.clone()).unwrap();
    let domain = msg.domain.clone();
    msg.user_addr = String::from("current");
    msg.site_addr = ws.address.address.clone();
    let res = block_on(ws.user_controller.send(msg.clone()))?;
    match res {
        Err(_) => command.respond("Not changed"),
        Ok(false) => {
            let user = get_current_user(ws)?;
            let current_cert = user.certs.get(&domain).unwrap();
            let body = format!(
                "Your current certificate: <b>{}/{}@{}</b>",
                current_cert.auth_type, current_cert.auth_user_name, domain,
            );
            let txt = format!(
                "Change it to {}/{}@{}",
                msg.auth_type, msg.auth_user_name, domain
            );
            let _ = ws.cmd(
                "confirm",
                json!([body, txt,]),
                Some(Box::new(move |ws, cmd| cert_add_confirm(ws, cmd))),
                Some(command.params.clone()),
            );
            command.command()
        }
        Ok(true) => {
            let _ = ws.cmd(
                "notification",
                json!([
                    "done",
                    format!(
                        "New certificate added: <b>{}/{}@{}</b>",
                        msg.auth_type, msg.auth_user_name, domain
                    )
                ]),
                None,
                None,
            );
            let msg = UserSetSiteCertRequest {
                user_addr: String::from("current"),
                site_addr: ws.address.address.clone(),
                provider: domain.clone(),
            };
            let _ = block_on(ws.user_controller.send(msg))?;
            ws.update_websocket(Some(json!(vec!["cert_changed", &domain])));
            command.respond("ok")
        }
    }
}

fn cert_add_confirm(ws: &mut ZeruWebsocket, cmd: &Command) -> Option<Result<Message, Error>> {
    let params = cmd.params.clone();
    let user = String::from("current");
    let mut add_msg: UserCertAddRequest = serde_json::from_value(params.clone()).unwrap();
    add_msg.user_addr = user.clone();
    add_msg.site_addr = ws.address.address.clone();

    let msg = UserCertDeleteRequest {
        user_addr: user.clone(),
        domain: add_msg.domain.clone(),
    };
    let _ = block_on(ws.user_controller.send(msg)).unwrap();
    let res = block_on(ws.user_controller.send(add_msg.clone())).unwrap();
    assert!(res.is_ok());
    assert!(res.unwrap());
    let _ = ws.cmd(
        "notification",
        json!([
            "done",
            format!(
                "Certificate changed to: <b>{}/{}@{}</b>",
                add_msg.auth_type, add_msg.auth_user_name, add_msg.domain
            )
        ]),
        None,
        None,
    );
    ws.update_websocket(Some(json!(vec!["cert_changed", &add_msg.domain])));
    Some(cmd.respond("ok"))
}

#[derive(Deserialize, Debug)]
struct CertSelectRequest {
    #[serde(default)]
    accepted_providers: Vec<String>,
    accepted_pattern: Option<String>,
    accept_any: bool,
}

pub fn handle_cert_select(ws: &mut ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let CertSelectRequest {
        accepted_providers,
        accepted_pattern,
        mut accept_any,
    } = extract_cert_select_params(cmd.params.clone());
    if !accept_any {
        accept_any = accepted_providers.is_empty() || accepted_pattern.is_none();
    };
    let site_data = block_on(ws.user_controller.send(UserSiteData {
        user_addr: String::from("current"),
        site_addr: ws.address.address.clone(),
    }))?
    .unwrap();
    let site_data = site_data.get(ws.address.address.as_str()).unwrap();
    let auth_addr = site_data.get_auth_pair().unwrap().auth_address;

    let mut providers = vec![];
    providers.push(vec![
        "".to_string(),
        "No certificate".to_string(),
        "".to_string(),
    ]);
    let mut active = String::new();
    let user = get_current_user(ws)?;
    for (provider, cert) in &user.certs {
        if auth_addr == cert.get_auth_pair().auth_address
            && Some(provider.clone()) == site_data.get_cert_provider()
        {
            active = provider.clone();
        }
        let title = format!("{}@{}", cert.auth_user_name, provider);
        let accepted_pattern_match = if let Some(accepted_pattern) = &accepted_pattern {
            let regex = regex::Regex::new(accepted_pattern);
            if let Ok(regex) = regex {
                regex.is_match(&provider)
            } else {
                false
            }
        } else {
            false
        };
        if accepted_providers.contains(&provider) || accept_any || accepted_pattern_match {
            providers.push(vec![provider.clone(), title, "".to_string()]);
        } else {
            providers.push(vec![provider.clone(), title, "disabled".to_string()]);
        }
    }
    let mut body = String::from( "<span style='padding-bottom: 5px; display: inline-block'>Select account you want to use in this site:</span>");

    for c in providers {
        let provider = &c[0];
        let account = &c[1];
        let css = &c[2];
        let (css, title) = if provider == &active {
            let css = format!("{} active", css);
            let title = format!("<b>{}</b> <small>currently selected</small>", account);
            (css, title)
        } else {
            (css.to_string(), format!("<b>{}</b>", account))
        };
        body += &format!(
            "<a href='#Select+account' class='select select-close cert {}' title='{}'>{}</a>",
            css, provider, title
        );
    }

    accepted_providers.iter().for_each(|provider| {
        if !user.certs.contains_key(provider.as_str()) {
            body += "<div style='background-color: #F7F7F7; margin-right: -30px'>";
            body += &format!(
                "<a href='/{}' target='_top' class='select'>
                            <small style='float: right; margin-right: 40px; margin-top: -1px'>
                            Register &raquo;</small>{}</a>",
                provider, provider
            );
            body += "</div>";
        }
    });

    let _ = ws.cmd(
        "notification",
        json!(["ask", body]),
        Some(Box::new(move |ws, cmd| Some(handle_cert_set(ws, cmd)))),
        None,
    );
    let script = notification_script_template(ws.next_message_id - 1);
    let _ = ws.cmd("injectScript", json!(script), None, None);
    cmd.inject_script()
}

fn extract_cert_select_params(params: Value) -> CertSelectRequest {
    if params.is_object() {
        serde_json::from_value(params).unwrap()
    } else if params.is_array() {
        let mut accepted_providers = vec![];
        let mut accepted_pattern = None;
        let mut accept_any = false;
        for param in params.as_array().unwrap() {
            if let Value::String(pattern) = param {
                accepted_pattern = Some(pattern.clone());
            } else if let Value::Bool(value) = param {
                accept_any = *value;
            } else if let Value::Array(providers) = param {
                for provider in providers {
                    if let Value::String(provider) = provider {
                        accepted_providers.push(provider.clone());
                    }
                }
            }
        }
        CertSelectRequest {
            accepted_providers,
            accepted_pattern,
            accept_any,
        }
    } else {
        unreachable!("Invalid params")
    }
}

fn notification_script_template(id: usize) -> String {
    format!(
        "
    $(\".notification .select.cert\").on(\"click\", function() {{
    $(\".notification .select\").removeClass('active')
    zeroframe.response({}, this.title)
    return false
    }})",
        id
    )
}

pub fn handle_site_info(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    let site_info_req = SiteInfoRequest {};
    let result = block_on(ws.site_addr.send(site_info_req))?;
    if result.is_err() {
        return Err(Error {
            error: String::from("Site info not found"),
        });
    }
    let mut site_info = result.unwrap();
    if let Some(site_info) = append_user_site_data(ws, &mut site_info) {
        if let Value::Object(params) = &command.params {
            if let Some(Value::String(path)) = params.get("file_status") {
                site_info.event = Some(json!(["file_done", path])); //TODO!: get file status
            }
        }
        command.respond(site_info)
    } else {
        Err(Error {
            error: String::from("Site info not found"),
        })
    }
}

pub fn append_user_site_data<'a>(
    ws: &ZeruWebsocket,
    site_info: &'a mut SiteInfo,
) -> Option<&'a mut SiteInfo> {
    if let Some(map) = block_on(ws.user_controller.send(UserSiteData {
        user_addr: String::from("current"),
        site_addr: site_info.address.clone(),
    }))
    .unwrap()
    {
        let user_site_data = map.values().last().unwrap();
        if let Some(provider) = user_site_data.get_cert_provider() {
            let user = get_current_user(ws).unwrap();
            if let Some(cert) = &user.certs.get(&provider) {
                site_info.cert_user_id = Some(format!("{}@{}", cert.auth_user_name, provider));
            }
        }
        if let Some(auth) = user_site_data.get_auth_pair() {
            site_info.auth_address = auth.auth_address;
        }
        if let Some(key) = user_site_data.get_privkey() {
            if !key.is_empty() {
                site_info.privatekey = true;
            }
        }
        #[cfg(debug_assertions)]
        {
            site_info.size_limit = 25;
            site_info.next_size_limit = 25;
        }
        return Some(site_info);
    }
    None
}

pub fn handle_db_query(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    trace!("Handling DBQuery {:?}", command.cmd);
    match &command.params {
        Value::Array(inner_path) => {
            if let Some(query) = inner_path[0].as_str() {
                let stmt_type = query.split_whitespace().next().unwrap();
                if stmt_type.to_uppercase() != "SELECT" {
                    return Err(Error {
                        error: String::from("Only SELECT queries are allowed"),
                    });
                }
                let params = inner_path.get(1).cloned();
                let res = block_on(ws.site_controller.send(DBQueryRequest {
                    address: ws.address.address.clone(),
                    query: query.to_string(),
                    params,
                }))
                .unwrap()
                .unwrap();
                return command.respond(res);
            }
            error!("{:?}", command);
            Err(Error {
                error: String::from("expecting query, failed to parse"),
            })
        }
        _ => {
            error!("{:?}", command);
            Err(Error {
                error: String::from("Invalid params"),
            })
        }
    }
}

pub fn handle_channel_join(
    ctx: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling ChannelJoin request");
    let channel = &command.params;
    let mut list = vec![];
    match channel {
        Value::Object(map) => {
            if let Some(Value::Array(channels)) = map.get("channels") {
                for channel in channels {
                    if let Some(channel) = channel.as_str() {
                        list.push(channel.to_string());
                    }
                }
            }
        }
        Value::String(channel) => {
            list.push(channel.to_string());
        }
        _ => {
            return Err(Error {
                error: String::from("Invalid params"),
            })
        }
    }
    ctx.address().do_send(RegisterChannels(list));
    command.respond("ok")
}

pub fn handle_site_list(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    trace!("Handling SiteList : {:?}", command.params);
    let mut connecting = false;
    if let Value::Object(map) = &command.params {
        if let Some(Value::Bool(value)) = map.get("connecting_sites") {
            connecting = *value;
        }
    }
    let sites = block_on(ws.site_controller.send(SiteInfoListRequest { connecting }))
        .unwrap()
        .unwrap();
    command.respond(sites)
}

pub fn handle_channel_join_all_site(
    _: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    debug!("Handling ChannelJoinAllsite request using dummy response");
    command.respond(String::from("ok"))
}

pub fn handle_site_sign(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_publish(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_reload(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_update(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_bad_files(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    trace!("Handling SiteBadFiles request");
    let bad_files = block_on(ws.site_controller.send(SiteBadFilesRequest {
        address: ws.address.address.clone(),
    }))?;
    cmd.respond(bad_files)
}

pub fn handle_site_list_modified_files(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_pause(ws: &mut ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SitePauseRequest {
        address: ws.address.address.clone(),
    }))?;
    ws.update_websocket(None);
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Paused")
}

pub fn handle_site_resume(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SiteResumeRequest {
        address: ws.address.address.clone(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Resumed")
}

pub fn handle_site_delete(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SiteDeleteRequest {
        address: ws.address.address.clone(),
    }))?;
    let data_res = block_on(ws.user_controller.send(UserSiteDataDeleteRequest {
        user_addr: String::from("current"),
        site_addr: ws.address.address.clone(),
    }))?;
    if data_res.is_err() | res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Deleted")
}

pub fn handle_site_set_settings_value(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_array().unwrap();
    let key = params[0].as_str().unwrap();
    if key != "modified_files_notification" {
        return Err(Error {
            error: format!("Can't change this key"),
        });
    }
    let res = block_on(ws.site_controller.send(SiteSetSettingsValueRequest {
        address: ws.address.address.clone(),
        key: key.to_string(),
        value: params[1].clone(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_add(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_str().unwrap();
    let res = block_on(ws.site_controller.send(SitePermissionAddRequest {
        address: ws.address.address.clone(),
        permission: params.to_string(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_remove(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_str().unwrap();
    let res = block_on(ws.site_controller.send(SitePermissionRemoveRequest {
        address: ws.address.address.clone(),
        permission: params.to_string(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_details(cmd: &Command) -> Result<Message, Error> {
    let key = cmd.params.as_str().unwrap();
    let details = SITE_PERMISSIONS_DETAILS
        .get(key)
        .cloned()
        .ok_or_else(|| Error {
            error: format!("Unknown permission: {}", key),
        });
    cmd.respond(details?)
}

#[derive(Serialize)]
pub struct OptionalLimitStats {
    pub limit: String,
    pub used: isize,
    pub free: isize,
}

pub fn _handle_optional_limit_stats(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    // TODO: replace dummy response with actual response
    warn!("Handling OptionalLimitStats with dummy response");
    let limit_stats = OptionalLimitStats {
        limit: String::from("10%"),
        used: 1000000,
        free: 4000000,
    };
    command.respond(limit_stats)
}
