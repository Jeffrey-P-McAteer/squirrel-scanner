
pub async fn is_proc_running(proc_name: &str) -> bool {
  if let Ok(procs) = procfs::process::all_processes() {
    for p in procs {
      if let Ok(p) = p {
        if let Ok(p_exe) = p.exe() {
          if let Some(p_file_name) = p_exe.file_name() {
            let p_file_name = p_file_name.to_string_lossy();
            if p_file_name == proc_name {
              return true;
            }
          }
        }
      }
    }
  }
  return false;
}


pub async fn do_nice_shutdown() {

  crate::PLEASE_EXIT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);

  tokio::task::spawn(async { // Shutdown webserver after a 350ms delay to allow other tasks to exit
    tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
    if let Some(current_system) = actix_web::rt::System::try_current() {
      current_system.stop();
    }
  });

}






