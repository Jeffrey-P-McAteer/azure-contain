
use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainConfig {
  pub container: ContainerBlock,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerBlock {
  pub name: String,

  #[serde(default = "default_empty_string")]
  pub welcome_msg: String,

  // One of nspawn, arch-chroot,
  #[serde(default = "default_runtime_hint")]
  pub runtime_hint: String,

  // Where data is on disk; first existing disk in list is used.
  // This design allows us to support multi-machine configs, and fall-back to other storage devices.
  pub disk_partuuids: Vec<String>,

  // Where it gets mounted; the tool will ensure part_subfolder.parent() == mountpoint of disk_partuuid
  pub part_subfolder: PathBuf,

  #[serde(default = "default_empty_string_vec")]
  pub install_setup_cmds: Vec<String>,

  #[serde(default = "default_empty_string_vec")]
  pub nspawn_addtl_args: Vec<String>,

  #[serde(default = "default_empty_string_vec")]
  pub fwd_env_vars: Vec<String>,

}

fn default_runtime_hint() -> String {
    "nspawn".to_string()
}

fn default_empty_string() -> String {
    "".to_string()
}

fn default_empty_string_vec() -> Vec<String> {
    vec![]
}


impl ContainerBlock {
  pub fn flag_path(&self, flag: &str) -> PathBuf {
    let mut flag_file_path = self.part_subfolder.clone();
    let mut file_name = flag_file_path.file_name().unwrap_or(std::ffi::OsStr::new(&self.name)).to_owned();
    file_name.push( &std::ffi::OsStr::new(flag) );
    flag_file_path.set_file_name(file_name);
    flag_file_path
  }
  pub fn get_disk_part_path(&self) -> String {
    for disk_partuuid in &self.disk_partuuids {
      let maybe_path = format!("/dev/disk/by-partuuid/{}", disk_partuuid);
      if std::path::Path::new(&maybe_path).exists() {
        return maybe_path;
      }
    }
    "/DEV/NO_DISK_IN_disk_partuuids_LIST_ATTACHED".to_string()
  }
}


