
// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]

// We use this in utils
#![feature(portable_simd)]

mod utils;
mod web;
mod camera;

static PLEASE_EXIT_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() -> Result<(), Box<dyn std::error::Error>> {
  if let Err(e) = os_prelude() {
    eprintln!("[ os_prelude ] {:?}", e);
  }

  let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async().await {
      eprintln!("[ main_async ] {:?}", e);
    }
  });

  if let Err(e) = os_epilogue() {
    eprintln!("[ os_epilogue ] {:?}", e);
  }

  Ok(())
}

fn os_prelude() -> Result<(), Box<dyn std::error::Error>>  {
  if let Ok(val) = std::env::var("NO_PRELUDE") {
    let val = val.to_string();
    if val.contains("t") || val.contains("T") || val.contains("1") {
      return Ok(());
    }
  }

  let unused = std::process::Command::new("chvt")
    .args(&["7"])
    .status();
  let unused = std::process::Command::new("sysctl") // From https://bbs.archlinux.org/viewtopic.php?id=284267
    .args(&["kernel.printk=0 4 0 4"])
    .status();

  Ok(())
}


fn os_epilogue() -> Result<(), Box<dyn std::error::Error>>  {
  if let Ok(val) = std::env::var("NO_PRELUDE") {
    let val = val.to_string();
    if val.contains("t") || val.contains("T") || val.contains("1") {
      return Ok(());
    }
  }

  let unused = std::process::Command::new("chvt")
    .args(&["1"])
    .status();

  Ok(())
}

#[allow(unreachable_code)]
async fn main_async() -> Result<(), Box<dyn std::error::Error>> {

  tokio::task::spawn(async {
    if let Err(e) = web::run_webserver_forever().await {
      eprintln!("[ run_webserver_forever ] {:?}", e);
    }
    // If the webserver goes down, everything should be going down.
    crate::PLEASE_EXIT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
  });

  loop {
    if crate::PLEASE_EXIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
      break;
    }

    if utils::is_proc_running("ffmpeg").await {
      eprintln!("Waiting for ffmpeg to exit, assuming it is doing development things...");
      tokio::time::sleep(tokio::time::Duration::from_millis(1250)).await;
      continue;
    }

    if let Err(e) = camera::camera_loop().await {
      eprintln!("[ camera_loop ] {:?}", e);
      let e_s = format!("{:?}", e);
      if e_s.contains("Interrupted") && e_s.contains("system") && e_s.contains("call") {
        // We see this on ctrl+c SIGTERM events, so play nice & decide to exit.
        utils::do_nice_shutdown().await;
      }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
  }

  Ok(())
}
