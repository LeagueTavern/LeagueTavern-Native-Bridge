use crate::ProcessInfo;
use libc::{c_int, c_void, pid_t, sysctl, CTL_KERN, KERN_PROCARGS2};
use std::ffi::CStr;
use std::mem;
use std::ptr;

// proc_listallpids: returns number of pids.
// int proc_listallpids(void * buffer, int buffersize);
extern "C" {
  fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
}

// proc_pidinfo
// int proc_pidinfo(int pid, int flavor, uint64_t arg,  void *buffer, int buffersize);
// PROC_PIDTBSDINFO = 1
const PROC_PIDTBSDINFO: c_int = 1;
const MAXCOMLEN: usize = 16;

#[repr(C)]
#[derive(Copy, Clone)]
struct proc_bsdinfo {
  pub pbi_flags: u32,
  pub pbi_status: u32,
  pub pbi_xstatus: u32,
  pub pbi_pid: u32,
  pub pbi_ppid: u32,
  pub pbi_uid: u32,
  pub pbi_gid: u32,
  pub pbi_ruid: u32,
  pub pbi_rgid: u32,
  pub pbi_svuid: u32,
  pub pbi_svgid: u32,
  pub pbi_rfu1: u32,
  pub pbi_comm: [u8; MAXCOMLEN],
  pub pbi_name: [u8; 2 * MAXCOMLEN],
  pub pbi_nfiles: u32,
  pub pbi_pgid: u32,
  pub pbi_pjobc: u32,
  pub pbi_ejobc: u32,
  pub pbi_tjobc: u32,
  pub pbi_start_tvsec: u64,
  pub pbi_start_tvusec: u64,
}

extern "C" {
  fn proc_pidinfo(
    pid: c_int,
    flavor: c_int,
    arg: u64,
    buffer: *mut c_void,
    buffersize: c_int,
  ) -> c_int;
}

pub fn find_processes_by_name(name: &str) -> Vec<ProcessInfo> {
  let mut results = Vec::new();

  unsafe {
    // First get size required
    let count_estimate = proc_listallpids(ptr::null_mut(), 0);
    if count_estimate <= 0 {
      return results;
    }

    // Allocate buffer with some extra space
    let buffer_size = (count_estimate as usize + 32) * mem::size_of::<pid_t>();
    let mut pids: Vec<pid_t> = vec![0; buffer_size / mem::size_of::<pid_t>()];

    let count = proc_listallpids(
      pids.as_mut_ptr() as *mut c_void,
      (pids.len() * mem::size_of::<pid_t>()) as c_int,
    );

    if count <= 0 {
      return results;
    }

    // Iterate pids
    // proc_listallpids returns number of PIDs, but be careful if buffer was too small (not checked here effectively as we oversized)
    let actual_count = std::cmp::min(count as usize, pids.len());

    for i in 0..actual_count {
      let pid = pids[i];
      if pid <= 0 {
        continue;
      }

      let mut bsdinfo: proc_bsdinfo = mem::zeroed();
      let ret = proc_pidinfo(
        pid,
        PROC_PIDTBSDINFO,
        0,
        &mut bsdinfo as *mut _ as *mut c_void,
        mem::size_of::<proc_bsdinfo>() as c_int,
      );

      if ret > 0 {
        // Check name
        // pbi_comm is max 16 chars, pbi_name is max 32 chars.
        // pbi_comm is usually the command name (executable filename).

        let comm_bytes = &bsdinfo.pbi_comm;
        let name_bytes = &bsdinfo.pbi_name;

        // Try pbi_comm first
        if let Ok(comm_str) = CStr::from_ptr(comm_bytes.as_ptr() as *const i8).to_str() {
          if comm_str == name {
            results.push(ProcessInfo {
              pid: pid as u32,
              name: comm_str.to_string(),
            });
            continue;
          }
        }

        // Try pbi_name if comm didn't match (pbi_name is sometimes longer)
        if let Ok(name_str) = CStr::from_ptr(name_bytes.as_ptr() as *const i8).to_str() {
          if name_str == name {
            results.push(ProcessInfo {
              pid: pid as u32,
              name: name_str.to_string(),
            });
          }
        }
      }
    }
  }

  results
}

pub fn find_process_by_pid(pid: u32) -> Option<ProcessInfo> {
  unsafe {
    let mut bsdinfo: proc_bsdinfo = mem::zeroed();
    let ret = proc_pidinfo(
      pid as c_int,
      PROC_PIDTBSDINFO,
      0,
      &mut bsdinfo as *mut _ as *mut c_void,
      mem::size_of::<proc_bsdinfo>() as c_int,
    );

    if ret > 0 {
      // Success
      // Prefer pbi_name if available and longer? pbi_comm is usually what `ps` shows as command.
      // Let's use pbi_comm for consistency with find_processes_by_name logic priority

      let comm_bytes = &bsdinfo.pbi_comm;
      if let Ok(comm_str) = CStr::from_ptr(comm_bytes.as_ptr() as *const i8).to_str() {
        return Some(ProcessInfo {
          pid,
          name: comm_str.to_string(),
        });
      }
    }
  }
  None
}

pub fn get_process_cmdline(pid: u32) -> Option<String> {
  unsafe {
    // KERN_PROCARGS2
    let mut mib = [CTL_KERN, KERN_PROCARGS2, pid as c_int];
    let mut size: usize = 0;

    // Get size
    let ret = sysctl(
      mib.as_mut_ptr(),
      3,
      ptr::null_mut(),
      &mut size,
      ptr::null_mut(),
      0,
    );
    if ret == -1 {
      return None;
    }

    let mut buffer: Vec<u8> = vec![0; size];
    let ret = sysctl(
      mib.as_mut_ptr(),
      3,
      buffer.as_mut_ptr() as *mut c_void,
      &mut size,
      ptr::null_mut(),
      0,
    );
    if ret == -1 {
      return None;
    }

    // Parse logic same as before
    if size < mem::size_of::<c_int>() {
      return None;
    }

    let argc_ptr = buffer.as_ptr() as *const c_int;
    let argc = *argc_ptr;

    let mut cursor = mem::size_of::<c_int>();

    let exec_path_end = buffer[cursor..].iter().position(|&c| c == 0)?;
    cursor += exec_path_end + 1;

    while cursor < buffer.len() && buffer[cursor] == 0 {
      cursor += 1;
    }

    if cursor >= buffer.len() {
      return Some(String::new());
    }

    let mut args = Vec::new();
    let mut current_arg_start = cursor;

    for _ in 0..argc {
      if current_arg_start >= buffer.len() {
        break;
      }
      if let Some(arg_end_offset) = buffer[current_arg_start..].iter().position(|&c| c == 0) {
        let arg_end = current_arg_start + arg_end_offset;
        let arg = String::from_utf8_lossy(&buffer[current_arg_start..arg_end]).to_string();
        args.push(arg);
        current_arg_start = arg_end + 1;
      } else {
        break;
      }
    }

    Some(args.join(" "))
  }
}
