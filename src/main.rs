
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

use std::io::Write;


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
  
  println!("container_config={:?}", container_config);

}

