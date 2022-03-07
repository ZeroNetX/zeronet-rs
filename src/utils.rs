use log::error;
use std::default::Default;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::core::error::Error;

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub fn atomic_write(
    dest: &Path,
    new_content: &[u8],
    content: &[u8],
    retry: bool,
) -> Result<bool, Error> {
    let mut options = OpenOptions::new();
    options.write(true).truncate(true);
    let mut file = options.open(dest)?;
    if let Err(e) = file.write_all(new_content) {
        error!("Error writing file: {:?}", e);
        //Possible data corruption in old file, overwrite with old content
        if retry {
            return atomic_write(dest, new_content, content, false);
        } else {
            file.write_all(content)?
        }
    }
    Ok(true)
}
