
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
    let first_arg = args[1].clone();
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(2)
      .build()
      .expect("Could not build tokio runtime!");

    return rt.block_on(container_manager(first_arg));
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

async fn container_manager(mut path_to_config: String) {
  if ! std::path::Path::new(&path_to_config).exists() {
    // Scan under /j/bins/azure-contain/containers for a file containing this & use that
    let mut containers_dir_o = dump_error_and_ret!( tokio::fs::read_dir("/j/bins/azure-contain/containers").await );
    while let Some(container_toml) = dump_error_and_ret!( containers_dir_o.next_entry().await ) {
      if container_toml.file_name().into_string().unwrap_or_default().contains(&path_to_config) {
        path_to_config = container_toml.path().into_os_string().into_string().unwrap_or_default();
        break;
      }
    }
  }

  println!("Reading {}", &path_to_config);
  let container_file_content = tokio::fs::read_to_string(path_to_config).await.expect("Could not read config file!");
  let container_config: ContainConfig = toml::from_str(&container_file_content).expect("Could not parse config!");
  // We don't really use outer layer much
  let container_config = container_config.container;
  
  // println!("container_config={:?}", container_config);

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

        let command_done_dir = std::path::PathBuf::from(&parent_disk_mount_pt).join( &container_config.flag_path(".install-steps-done") );
        dump_error!(  tokio::fs::create_dir_all(command_done_dir).await );

        for (command_i, command_str) in container_config.install_setup_cmds.iter().enumerate() {
          let mut command_done_flag = std::path::PathBuf::from(&parent_disk_mount_pt).join( container_config.flag_path(".install-steps-done") );
          command_done_flag.push(format!("{}", command_i));
          if command_done_flag.exists() {
            println!("Skipping step {} because {:?} exists: {}", command_i, &command_done_flag, &command_str );
            continue;
          }

          if command_str.starts_with("SH_IN_CONTAINER") {
            // We need to pass the end of this as a bare string to /bin/sh -c within the container.
            // Easier to handle here than escape in config.toml
            let command_str = command_str.replace("SH_IN_CONTAINER:", "");
            let command = dump_error_and_ret!( SimpleCurlyFormat.format(&command_str, &install_cmd_vars) );
            let command = command.trim();

            println!("[Install Cmd] systemd-nspawn -D \"{}\" sh -c \"{}\"", &ref_to_container_root_dir, &command);
            dump_error_and_ret!(
              tokio::process::Command::new("sudo")
                .args(&["-n", "systemd-nspawn", "-D", &ref_to_container_root_dir, "sh", "-c", &command])
                .status()
                .await
            );
          }
          else {
            let command = dump_error_and_ret!( SimpleCurlyFormat.format(&command_str, &install_cmd_vars) );
            let command = command.trim();
            println!("[Install Cmd] {}", &command);
            dump_error_and_ret!(
              tokio::process::Command::new("sudo")
                .args(&["-n", "sh", "-c", &command ])
                .status()
                .await
            );
          }

          // Command done!
          dump_error!( tokio::fs::write(&command_done_flag, "done").await );

        }

        dump_error!( tokio::fs::write(&install_completed_flag, "done").await );
      }

      println!("{} exists, running with runtime hint {}!", install_completed_flag.display(), &container_config.runtime_hint );

      let mut args: Vec<String> = vec![];
      args.push("-n".to_string()); // for sudo

      if container_config.runtime_hint.contains("nspawn") {
      
        args.push("systemd-nspawn".to_string()); // begin nspawn command
        args.push("-D".to_string());
        args.push(ref_to_container_root_dir.to_string());

        args.push(format!("--machine={}", &container_config.name ));
        args.push(format!("--hostname={}", &container_config.name ));

        for fwd_env_var in container_config.fwd_env_vars.iter() {
          if let Ok(var_value) = std::env::var(&fwd_env_var) {
            let param = format!("--setenv={}={}", fwd_env_var, var_value);
            args.push(param);
          }
        }

        for nspawn_addtl_arg in container_config.nspawn_addtl_args.iter() {
          // Skip --bind args to files which do not exist (such as /dev/nvidia0)
          if nspawn_addtl_arg.contains("--bind=") && nspawn_addtl_arg.contains("=") {
            let host_path = nspawn_addtl_arg.split("=").collect::<Vec<&str>>()[1];
            let host_path = host_path.split(":").collect::<Vec<&str>>()[0];
            if ! std::path::PathBuf::from(host_path).exists() {
              println!("Ignoring arg {} because {} does not exist!", &nspawn_addtl_arg, &host_path);
              continue;
            }
          }
          args.push(nspawn_addtl_arg.to_string());
        }

        // Finally, add on addtl args
        let sys_args: Vec<String> = std::env::args().collect();
        if sys_args.len() > 2 {
          for addtl_arg in &sys_args[2..] {
            args.push(addtl_arg.to_owned());
          }
        }


      }
      else if container_config.runtime_hint.contains("arch-chroot")  {

        args.push("arch-chroot".to_string());

        args.push(ref_to_container_root_dir.to_string());

      }
      else {
        println!("Unknown runtime hint!");
        return;
      }

      let container_cmd_s = args.join(" ");
      println!("[Run Cmd] sudo {}", container_cmd_s);

      println!("");
      println!("{}", &container_config.welcome_msg);
      println!("");
      

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


