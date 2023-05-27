
#![allow(dead_code)]
#![allow(unused_imports)]

use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

use std::io::Write;

use dynfmt::{Format, SimpleCurlyFormat};

mod structs;
use structs::*;

#[macro_use]
pub mod macros;


fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    return dump_help();
  }
  else {
    let first_arg = &args[1];

    if first_arg.ends_with(".toml") {
      let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("Could not build tokio runtime!");

      return rt.block_on(container_manager(first_arg));
    }
    else {
      println!("TODO build client system to handle arg {}", first_arg);
    }
  }
}

fn dump_help() {
  println!(r#"Usage:
  {exe} /path/to/container.toml
    
    Runs the Container

  {exe} TODO more runtime container control stuff

"#,
  exe=std::env::current_exe().unwrap_or(std::path::PathBuf::from("/dev/null")).display()
);
}

async fn container_manager(path_to_config: &str) {
  println!("Reading {}", &path_to_config);
  let container_file_content = tokio::fs::read_to_string(path_to_config).await.expect("Could not read config file!");
  let container_config: ContainConfig = toml::from_str(&container_file_content).expect("Could not parse config!");
  // We don't really use outer layer much
  let container_config = container_config.container;
  
  println!("container_config={:?}", container_config);

  // Check if container_config.btrfs_partuuid is mounted, if not exit!
  match get_mount_pt_of( &container_config.get_disk_part_path() ).await {
    None => {
      println!("Please mount {:?} someplace!", &container_config.get_disk_part_path() );
      return;
    }
    Some(parent_disk_mount_pt) => {
      let container_root_dir = std::path::PathBuf::from(&parent_disk_mount_pt).join(&container_config.part_subfolder);
      if ! container_root_dir.exists() {
        dump_error!( tokio::fs::create_dir_all(&container_root_dir).await );
      }

      let mut container_root_has_files = false;
      let mut container_root_fs_dir_o = dump_error_and_ret!( tokio::fs::read_dir(&container_root_dir).await );
      while let Some(_child) = dump_error_and_ret!( container_root_fs_dir_o.next_entry().await ) {
        container_root_has_files = true;
      }

      println!("container_root_dir = {}", container_root_dir.display());

      let ref_to_container_root_dir = container_root_dir.to_string_lossy();

      let install_completed_flag = std::path::PathBuf::from(&parent_disk_mount_pt).join( &container_config.flag_path(".install-completed") );
      if !container_root_has_files || !install_completed_flag.exists() {
        // Run all install commands as root
        let mut install_cmd_vars: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        install_cmd_vars.insert("container_root_dir", &ref_to_container_root_dir);

        for command_str in container_config.install_setup_cmds.iter() {
          let command = dump_error_and_ret!( SimpleCurlyFormat.format(&command_str, &install_cmd_vars) );
          println!("[Install Cmd] {}", &command);
          dump_error!(
            tokio::process::Command::new("sudo")
              .args(&["-n", "sh", "-c", &command ])
              .status()
              .await
          );
        }

        dump_error!( tokio::fs::write(&install_completed_flag, "done").await );
      }

      println!("{} exists, booting!", install_completed_flag.display());

      let mut args: Vec<&str> = vec![];
      args.push("-n"); // for sudo

      args.push("systemd-nspawn"); // begin nspawn command
      args.push("-D");
      args.push(&ref_to_container_root_dir);


      let container_cmd_s = args.join(" ");
      println!("[Run Cmd] {}", container_cmd_s);

      dump_error!(
        tokio::process::Command::new("sudo")
          .args(&args)
          .status()
          .await
      );
    }
  }
}


async fn get_mount_pt_of(device_path: &str) -> Option<std::path::PathBuf> {
  if let Ok(device_path) = tokio::fs::canonicalize(device_path).await {
    if let Ok(info) = mountinfo::MountInfo::new() {
      for mount_pt in info.mounting_points {
        //println!("mount_pt={:?}", mount_pt);
        if std::path::PathBuf::from(mount_pt.what) == device_path {
          return Some(mount_pt.path);
        }
      }
    }
  }
  return None;
}

async fn is_mounted(directory_path: &str) -> bool {
  if let Ok(info) = mountinfo::MountInfo::new() {
    return info.is_mounted(directory_path);
  }
  return false;
}


