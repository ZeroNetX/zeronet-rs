use crate::{
    core::{io::SiteIO, site::Site},
    environment::ENV,
};
use std::path::PathBuf;

pub trait BlockStorage: SiteIO {
    fn use_block_storage() -> bool;

    fn get_block_storage_path(&self) -> PathBuf;

    fn get_block_file_path(&self, block_id: &str) -> PathBuf;
}

#[cfg(feature = "blockstorage")]
impl BlockStorage for Site {
    fn use_block_storage() -> bool {
        ENV.use_block_storage
    }

    fn get_block_storage_path(&self) -> PathBuf {
        self.data_path.join("blockstorage")
    }

    fn get_block_file_path(&self, block_id: &str) -> PathBuf {
        self.get_block_storage_path().join(block_id)
    }
}
