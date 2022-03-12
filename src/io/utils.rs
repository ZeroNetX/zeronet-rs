use std::path::Path;

use sha2::Digest;
use sha2::Sha512;
use tokio::{fs::File, io::AsyncReadExt};

use crate::core::error::Error;

pub async fn get_file_hash(path: impl AsRef<Path>) -> Result<(usize, String), Error> {
    let file = File::open(&path).await;
    if let Err(_err) = file {
        return Err(Error::FileNotFound(format!(
            "File Not Found at Path {:?}",
            path.as_ref()
        )));
    }
    let mut buf = Vec::new();
    file.unwrap().read_to_end(&mut buf).await?;
    let len = buf.len();
    let digest = Sha512::digest(buf);
    let hash = format!("{:x}", digest)[..64].to_string();
    Ok((len, hash))
}

pub async fn check_file_integrity(path: impl AsRef<Path>, hash_str: String) -> Result<(), Error> {
    let hash = get_file_hash(path).await?;
    if hash_str != hash.1 {
        return Err(Error::Err("File integrity check failed".into()));
    }
    Ok(())
}
