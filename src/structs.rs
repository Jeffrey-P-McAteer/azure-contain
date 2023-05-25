
use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainConfig {
  pub container: ContainerBlock,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerBlock {
  pub name: String,
  
  // Where data is on disk
  pub btrfs_partuuid: String,
  pub btrfs_subvolume: usize,

  // Where it gets mounted
  pub disk_path: PathBuf,


}

impl ContainerBlock {
  pub fn flag_path(&self, flag: &str) -> PathBuf {
    let mut flag_file_path = self.disk_path.clone();
    let mut file_name = flag_file_path.file_name().unwrap_or(std::ffi::OsStr::new(&self.name)).to_owned();
    file_name.push( &std::ffi::OsStr::new(flag) );
    flag_file_path.set_file_name(file_name);
    flag_file_path
  }
}


