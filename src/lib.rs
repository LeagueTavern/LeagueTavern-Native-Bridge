#![deny(clippy::all)]

use napi_derive::napi;

#[napi(object)]
#[derive(Debug, Clone)]
pub struct ProcessInfo {
  pub pid: u32,
  pub name: String,
}

#[cfg(windows)]
#[path = "process/windows.rs"]
mod native;

#[cfg(target_os = "macos")]
#[path = "process/macos.rs"]
mod native;

#[cfg(not(any(windows, target_os = "macos")))]
mod native {
  use super::ProcessInfo;
  pub fn find_processes_by_name(_name: &str) -> Vec<ProcessInfo> {
    vec![]
  }
  pub fn find_process_by_pid(_pid: u32) -> Option<ProcessInfo> {
    None
  }
  pub fn get_process_cmdline(_pid: u32) -> Option<String> {
    None
  }
}

#[napi]
pub fn find_processes_by_name(name: String) -> Vec<ProcessInfo> {
  native::find_processes_by_name(&name)
}

#[napi]
pub fn find_process_by_pid(pid: u32) -> Option<ProcessInfo> {
  native::find_process_by_pid(pid)
}

#[napi]
pub fn get_process_cmdline(pid: u32) -> Option<String> {
  native::get_process_cmdline(pid)
}
