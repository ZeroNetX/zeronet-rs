use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use futures::future::join_all;
use log::*;
use serde_bytes::ByteBuf;
use serde_json::Value;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

use zerucontent::{Content, File as ZFile};

use crate::{
    core::{error::*, io::*, peer::*, site::*},
    discovery::tracker::IpPort,
    environment::ENV,
    io::utils::check_file_integrity,
    plugins::BlockStorage,
    protocol::{api::Request, Protocol},
};

impl Site {
    pub async fn create(&mut self, addr_idx: u32, private_key: &str) -> Result<(), Error> {
        let mut content = Content::create(self.address(), addr_idx);
        content.zeronet_version = ENV.version.clone();
        content.signs_required = 1;
        content.signers_sign =
            zeronet_cryptography::sign(format!("1:{}", self.address()), private_key)?;
        self.modify_content(None, content);
        self.add_file_data(private_key).await?;
        Ok(())
    }

    //TODO? Move this to templates module
    async fn add_file_data(&mut self, private_key: &str) -> Result<(), Error> {
        let data_dir = &*ENV.data_path;
        let site_dir = data_dir.join(&self.address());
        fs::create_dir_all(&site_dir).await?;
        let index_path = site_dir.join("index.html");
        let mut file = File::create(&index_path).await?;
        file.write_all(b"Welcome to World of DecentNet, A Peer to Peer Framework for Decentralised App and Services!")
            .await?;
        let _ = &self.add_file_to_content("index.html".into()).await?;
        self.sign_content(None, private_key).await?;
        self.save_content(None).await?;
        Ok(())
    }
    #[async_recursion::async_recursion]
    async fn download_file_from_peer(
        &self,
        inner_path: String,
        file: Option<ZFile>,
        peer: &mut Peer,
    ) -> Result<ByteBuf, Error> {
        let mut file_size = 0;
        if let Some(file) = &file {
            file_size = file.size;
        }
        let def_read_bytes = 512 * 1024;
        if file_size > def_read_bytes {
            let mut bytes = ByteBuf::new();
            let mut downloaded = 0;
            while downloaded != file_size {
                let message = Protocol::new(peer.connection_mut().unwrap())
                    .get_file(
                        self.address(),
                        inner_path.clone(),
                        file_size,
                        downloaded,
                        Some(def_read_bytes),
                    )
                    .await;
                if let Err(e) = &message {
                    return Err(format!("Error Downloading File from Peer, Error : {:?}", e)
                        .as_str()
                        .into());
                } else {
                    let bytes_downloaded = message.unwrap().body;
                    downloaded += bytes_downloaded.len();
                    println!("Downloaded File from Peer : {}, {}", inner_path, downloaded);
                    bytes.extend_from_slice(&bytes_downloaded);
                }
            }
            Ok(bytes)
        } else {
            let message = Protocol::new(peer.connection_mut().unwrap())
                .get_file(self.address(), inner_path.clone(), file_size, 0, None)
                .await;
            if let Err(e) = &message {
                Err(format!("Error Downloading File from Peer, Error : {:?}", e)
                    .as_str()
                    .into())
            } else {
                let msg = message.unwrap();
                let bytes = msg.body;
                if bytes.len() == msg.size {
                    Ok(bytes)
                } else {
                    //TODO: Optimize this by reusing downloaded buffer
                    self.download_file_from_peer(
                        inner_path,
                        Some(ZFile {
                            sha512: "".into(),
                            size: msg.size,
                        }),
                        peer,
                    )
                    .await
                }
            }
        }
    }

    pub async fn need_file(
        &self,
        inner_path: String,
        file: Option<ZFile>,
        _peer: Option<Peer>,
    ) -> Result<bool, Error> {
        self.download_file(inner_path, file, _peer).await
    }

    async fn download_file(
        &self,
        inner_path: String,
        file: Option<ZFile>,
        _peer: Option<Peer>,
    ) -> Result<bool, Error> {
        let (parent, path) = if let Some(file) = file.clone() {
            if cfg!(feature = "blockstorage") && Self::use_block_storage() {
                let file_path = BlockStorage::get_block_file_path(self, &file.sha512);
                let parent = BlockStorage::get_block_storage_path(self);
                (parent, file_path)
            } else {
                let path = self.site_path().join(&inner_path);
                (path.parent().unwrap().into(), path)
            }
        } else {
            let path = self.site_path().join(&inner_path);
            (path.parent().unwrap().into(), path)
        };
        if !parent.is_dir() {
            fs::create_dir_all(parent).await?;
        }
        if path.is_file() {
            //TODO! Verify file integrity here.
            return Ok(true);
        }
        //TODO!: Download from multiple peers
        let mut peer = self.peers.values().next().unwrap().clone();
        let bytes = Self::download_file_from_peer(self, inner_path, file, &mut peer).await?;
        let mut file = File::create(path).await?;
        file.write_all(&bytes).await?;

        Ok(true)
    }

    async fn download_site_files(&self) -> Result<(), Error> {
        let files = self.content(None).unwrap().files;
        let mut tasks = Vec::new();
        let mut inner_paths = Vec::new();
        for (inner_path, file) in files {
            inner_paths.push(inner_path.clone());
            let task = self.download_file(inner_path, Some(file), None);
            tasks.push(task);
        }
        let includes = self.content(None).unwrap().includes;
        for (inner_path, _file) in includes {
            inner_paths.push(inner_path.clone());
            let task = self.download_file(inner_path, None, None);
            tasks.push(task);
        }
        //TODO!: Other client may not have an up-to-date site files
        let user_files = self.fetch_changes(1421043090).await?;
        //TODO!: Check for storage Permission
        let mut user_data_files = Vec::new();
        for (inner_path, _file) in user_files {
            if inner_paths.contains(&inner_path) {
                continue;
            }
            user_data_files.push(inner_path.clone());
            let task = self.download_file(inner_path, None, None);
            tasks.push(task);
        }
        let mut res = join_all(tasks).await;
        let errs = res.drain_filter(|res| !res.is_ok()).collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err);
        }

        let user_data = user_data_files
            .iter()
            .map(|path| self.load_content_from_path(path.clone()))
            .collect::<Vec<_>>();
        let mut content_res = join_all(user_data).await;
        let errs = content_res
            .drain_filter(|res| !res.is_ok())
            .collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err.err());
        }
        let mut files = vec![];
        content_res.iter_mut().for_each(|content| {
            let content = content.as_ref().unwrap();
            let path = Path::new(&content.inner_path);
            if let Some(parent) = path.parent() {
                let files_inner = content.files.clone();
                for (path, file) in files_inner {
                    files.push(self.download_file(
                        parent.join(path).to_str().unwrap().to_owned(),
                        Some(file),
                        None,
                    ));
                }
            }
        });
        let mut res = join_all(files).await;
        let errs = res.drain_filter(|res| !res.is_ok()).collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err);
        }

        Ok(())
    }

    pub async fn load_content(&mut self) -> Result<bool, Error> {
        let buf = fs::read(self.content_path()).await?;
        let buf = ByteBuf::from(buf);
        let content = Content::from_buf(buf).unwrap();
        self.modify_content(None, content);
        let res = self.verify_content(None).is_ok();
        Ok(res)
    }

    pub async fn verify_files(&self, content_only: bool) -> Result<bool, Error> {
        if self.content(None).is_none() {
            Err(Error::Err("No content to verify".into()))
        } else {
            if !content_only {
                let res = self.check_site_integrity().await?;
                if !res.is_empty() {
                    return Err(Error::Err(format!(
                        "Site Integrity Check Failed: {:?}",
                        res
                    )));
                }
            }
            let content = self.content(None).unwrap();
            //TODO! Verify inner content also
            let verified = content.verify((&self.address()).clone());
            if !verified {
                return Err(Error::Err(format!(
                    "Content verification failed for {}",
                    self.address()
                )));
            } else {
                Ok(verified)
            }
        }
    }

    pub async fn check_site_integrity(&self) -> Result<Vec<(String, zerucontent::File)>, Error> {
        let content = self.content(None).unwrap();
        let files = content.files;
        let mut tasks = Vec::new();
        for (inner_path, file) in files {
            let hash = file.sha512.clone();
            let (site_path, inner_path) =
                if cfg!(feature = "blockstorage") && Self::use_block_storage() {
                    let path = BlockStorage::get_block_storage_path(self);
                    (path, hash.clone())
                } else {
                    (self.site_path(), inner_path)
                };
            let task = check_file_integrity(site_path, inner_path, hash);
            tasks.push(task);
        }
        //TODO!: Verify includes, user data files
        let mut res = join_all(tasks).await;
        let errs = res.drain_filter(|res| res.is_err()).collect::<Vec<_>>();
        for err in &errs {
            println!("{:?}", err);
        }
        if !errs.is_empty() {
            return Err(Error::Err("Site integrity check failed".into()));
        }
        let res = res
            .iter_mut()
            .filter_map(|r| {
                let r = &(r);
                if let Ok(r) = r {
                    let (v, i, h) = r;
                    if *v {
                        None
                    } else {
                        Some((i.clone(), h.clone()))
                    }
                } else {
                    unreachable!()
                }
            })
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub async fn fetch_changes(&self, since: usize) -> Result<HashMap<String, usize>, Error> {
        //TODO!: Download from multiple peers
        let mut peer = self.peers.values().next().unwrap().clone();
        let message = Protocol::new(peer.connection_mut().unwrap())
            .list_modified(self.address(), since)
            .await?;
        let changes = message.modified_files;
        Ok(changes)
    }

    pub async fn get_peers(&self) -> Result<Vec<Peer>, Error> {
        let mut peers = Vec::new();
        for peer in self.peers.values() {
            peers.push(peer.clone());
        }
        Ok(peers)
    }

    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.address().to_string(), peer);
    }

    pub async fn fetch_peers(&mut self) -> Result<Vec<String>, Error> {
        let addr = (&self.address()).clone();
        let mut peer = self.peers.values().next().unwrap().clone();
        let res = Protocol::new((peer.connection_mut()).unwrap())
            .pex(addr.clone())
            .await?
            .peers
            .iter()
            .map(|bytes| {
                let pair = IpPort::from_bytes(bytes.as_ref());
                pair.first().unwrap().to_string()
            })
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub async fn update(&mut self, inner_path: &str, diff: Option<HashMap<String, Vec<Value>>>) {
        let addr = (&self.address()).clone();
        let path = self.site_path().join(inner_path);
        let modified = self.content(None).unwrap().modified;
        let peer = self.peers.values_mut().next().unwrap();
        let content = fs::read(path).await.unwrap();
        let res = Protocol::new(peer.connection_mut().unwrap())
            .update(
                addr,
                inner_path.to_owned(),
                ByteBuf::from(content),
                diff.unwrap_or_default(),
                modified,
            )
            .await;
        if let Err(err) = res {
            error!("{:?}", err);
        }
    }
}

#[async_trait::async_trait]
impl SiteIO for Site {
    fn site_path(&self) -> PathBuf {
        self.data_path.join(self.address())
    }

    fn content_path(&self) -> PathBuf {
        self.site_path().join("content.json")
    }

    fn get_path(&self, inner_path: &str) -> Result<PathBuf, Error> {
        if inner_path.starts_with("../") {
            return Err(Error::Err(format!("Path Not Allowed: {}", inner_path)));
        }
        let path = self.site_path().join(inner_path);
        if path.exists() {
            Ok(path)
        } else {
            Err(Error::Err(format!(
                "Path not found: {}",
                path.to_string_lossy()
            )))
        }
    }

    fn get_inner_path(&self, path: &str) -> Result<PathBuf, Error> {
        if !path.starts_with(self.data_path.to_str().unwrap()) {
            return Err(Error::Err(format!("Path Not Allowed: {}", path)));
        } else {
            let path = PathBuf::from(path);
            if path.exists() {
                if self.data_path == path {
                    Ok(self.data_path.clone())
                } else {
                    Ok(path.strip_prefix(self.data_path.clone()).unwrap().into())
                }
            } else {
                Err(Error::Err(format!(
                    "Path not found: {}",
                    path.to_string_lossy()
                )))
            }
        }
    }

    async fn init_download(&mut self) -> Result<bool, Error> {
        if !&self.site_path().is_dir() {
            fs::create_dir_all(self.site_path()).await?;
        }
        let content_exists = self.content_path().is_file();
        if !content_exists {
            Self::download_file(self, "content.json".into(), None, None).await?;
        }
        let verified = self.load_content().await?;
        if verified {
            let _ = self.download_site_files().await;
        }
        self.verify_files(false).await?;
        Ok(verified)
    }

    async fn load_storage(_path: &str) -> Result<bool, Error> {
        unimplemented!()
    }

    async fn save_storage(&self) -> Result<bool, Error> {
        unimplemented!()
    }
}
