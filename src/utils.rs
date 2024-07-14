
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

