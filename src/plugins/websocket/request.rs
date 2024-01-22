use serde::{Deserialize, Serialize};

use super::{error::Error, response::Message};
use crate::utils::is_default;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CommandType {
    Response(ResponseCommandType),
    UiServer(UiServerCommandType),
    Admin(AdminCommandType),
    Plugin(PluginCommands),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "camelCase")]
pub enum ResponseCommandType {
    Response,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum UiServerCommandType {
    AnnouncerInfo,
    CertAdd,
    CertSelect,
    ChannelJoin,
    DbQuery,
    DirList,
    FileDelete,
    FileGet,
    FileList,
    FileNeed,
    FileQuery,
    FileRules,
    FileWrite,
    Ping,
    ServerInfo,
    SiteInfo,
    SitePublish,
    SiteReload,
    SiteSign,
    SiteUpdate,
    SiteBadFiles,
    SiteListModifiedFiles,
    UserGetSettings,
    UserSetSettings,
    UserGetGlobalSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AdminCommandType {
    // Admin commands
    As,
    CertList,
    CertSet,
    ChannelJoinAllsite,
    ConfigSet,
    ServerPortcheck,
    ServerShutdown,
    ServerUpdate,
    ServerErrors,
    ServerGetWrapperNonce,
    ServerShowdirectory,
    SiteAdd,
    SiteClone,
    SiteList,
    SitePause,
    SiteResume,
    SiteDelete,
    SiteSetLimit,
    SiteSetSettingsValue,
    PermissionAdd,
    PermissionRemove,
    PermissionDetails,
    UserSetGlobalSettings,
    AnnouncerStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PluginCommands {
    // Bigfile
    BigFileUploadInit,
    // Chart
    ChartDbQuery,
    ChartGetPeerLocations,
    // Cors
    CorsPermission,
    // Multiuser
    UserLoginForm,
    UserShowMasterSeed,
    // CryptMessage
    UserPublickey,
    EciesEncrypt,
    EciesDecrypt,
    AesEncrypt,
    AesDecrypt,
    // Newsfeed
    FeedFollow,
    FeedListFollow,
    FeedQuery,
    // MergerSite
    MergerSiteAdd,
    MergerSiteDelete,
    MergerSiteList,
    // Mute
    MuteAdd,
    MuteRemove,
    MuteList,
    // OptionalManager
    OptionalFileList,
    OptionalFileInfo,
    OptionalFilePin,
    OptionalFileUnpin,
    OptionalFileDelete,
    OptionalLimitStats,
    OptionalLimitSet,
    OptionalHelpList,
    OptionalHelp,
    OptionalHelpRemove,
    OptionalHelpAll,

    FilterIncludeList,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub cmd: CommandType,
    pub params: serde_json::Value,
    pub id: isize,
    #[serde(skip_serializing_if = "is_default", default)]
    pub wrapper_nonce: String,
}

impl Command {
    pub fn respond<T: Serialize>(&self, body: T) -> Result<Message, Error> {
        let resp = Message::new(self.id, serde_json::to_value(body)?);
        Ok(resp)
    }

    pub fn inject_script<T: Serialize>(&self, id: isize, body: T) -> Result<Message, Error> {
        let resp = Message::inject_script(id, serde_json::to_value(body)?);
        Ok(resp)
    }

    pub fn command(&self) -> Result<Message, Error> {
        let resp = Message::command();
        Ok(resp)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CommandResponse {
    pub cmd: String,
    pub id: isize,
    pub to: isize,
    pub result: serde_json::Value,
}

impl CommandResponse {
    pub fn respond<T: Serialize>(&self, body: T) -> Result<Message, Error> {
        let resp = Message::new(self.id, serde_json::to_value(body)?);
        Ok(resp)
    }
}
