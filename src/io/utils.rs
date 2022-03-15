use std::path::{Path, PathBuf};

use sha2::{Digest, Sha512};
use tokio::{fs::File, io::AsyncReadExt};
use zerucontent::File as ZFile;

use crate::core::error::Error;

pub async fn get_zfile_info(path: impl AsRef<Path>) -> Result<ZFile, Error> {
    let file = File::open(&path).await;
    if let Err(_err) = file {
        return Err(Error::FileNotFound(format!(
            "File Not Found at Path {:?}",
            path.as_ref()
        )));
    }
    let mut buf = Vec::new();
    file.unwrap().read_to_end(&mut buf).await?;
    let size = buf.len();
    let digest = Sha512::digest(buf);
    let sha512 = format!("{:x}", digest)[..64].to_string();
    Ok(ZFile { size, sha512 })
}

pub async fn check_file_integrity(
    site_path: PathBuf,
    inner_path: String,
    hash_str: String,
) -> Result<(bool, String, ZFile), Error> {
    let hash = get_zfile_info(site_path.join(&inner_path)).await?;
    if hash_str != hash.sha512 {
        return Ok((false, inner_path, hash));
    }
    Ok((true, inner_path, hash))
}
