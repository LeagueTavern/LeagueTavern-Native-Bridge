use crate::ProcessInfo;
// use ntapi::ntpebteb::PEB;
// use ntapi::ntpsapi::PROCESS_BASIC_INFORMATION;
// use ntapi::ntrtl::RTL_USER_PROCESS_PARAMETERS;
// use windows::Win32::Foundation::{CloseHandle, HANDLE};
// use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;

use std::ffi::c_void;
use windows::Win32::Foundation::{CloseHandle, HANDLE, UNICODE_STRING};
use windows::Win32::System::Diagnostics::ToolHelp::{
  CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

// use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

#[link(name = "ntdll")]
extern "system" {
  fn NtQueryInformationProcess(
    ProcessHandle: HANDLE,
    ProcessInformationClass: u32,
    ProcessInformation: *mut c_void,
    ProcessInformationLength: u32,
    ReturnLength: *mut u32,
  ) -> i32;
}

pub fn find_processes_by_name(name: &str) -> Vec<ProcessInfo> {
  let mut results = Vec::new();

  let name_lower = name.to_ascii_lowercase();
  let target_name = if name_lower.ends_with(".exe") {
    name_lower
  } else {
    format!("{}.exe", name_lower)
  };

  let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
  if snapshot.is_err() {
    return results;
  }
  let snapshot = snapshot.unwrap();

  let mut entry = PROCESSENTRY32W::default();
  entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

  if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
    loop {
      let process_name = String::from_utf16_lossy(&entry.szExeFile)
        .trim_matches(char::from(0))
        .to_string();

      if process_name.eq_ignore_ascii_case(&target_name) {
        results.push(ProcessInfo {
          pid: entry.th32ProcessID,
          name: process_name,
        });
      }

      if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
        break;
      }
    }
  }

  unsafe {
    let _ = CloseHandle(snapshot);
  }

  results
}

pub fn find_process_by_pid(pid: u32) -> Option<ProcessInfo> {
  let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
  if snapshot.is_err() {
    return None;
  }
  let snapshot = snapshot.unwrap();

  let mut entry = PROCESSENTRY32W::default();
  entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

  let mut result = None;

  if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
    loop {
      if entry.th32ProcessID == pid {
        let process_name = String::from_utf16_lossy(&entry.szExeFile)
          .trim_matches(char::from(0))
          .to_string();
        result = Some(ProcessInfo {
          pid: entry.th32ProcessID,
          name: process_name,
        });
        break;
      }

      if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
        break;
      }
    }
  }
  unsafe {
    let _ = CloseHandle(snapshot);
  };
  result
}

pub fn get_process_cmdline(pid: u32) -> Option<String> {
  unsafe {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

    let mut return_length: u32 = 0;
    // ProcessCommandLineInformation = 60
    let process_command_line_information = 60;

    // Query size
    let status = NtQueryInformationProcess(
      handle,
      process_command_line_information,
      std::ptr::null_mut(),
      0,
      &mut return_length,
    );

    // STATUS_BUFFER_OVERFLOW = 0x80000005
    // STATUS_BUFFER_TOO_SMALL = 0xC0000023
    // STATUS_INFO_LENGTH_MISMATCH = 0xC0000004
    // Success (0) is also possible if empty?
    if status != 0
      && status != 0xC0000004u32 as i32
      && status != 0xC0000023u32 as i32
      && status != 0x80000005u32 as i32
    {
      let _ = CloseHandle(handle);
      return None;
    }

    if return_length == 0 {
      let _ = CloseHandle(handle);
      return None;
    }

    // Allocate buffer
    let mut buffer: Vec<u8> = Vec::with_capacity(return_length as usize);
    // Safety: we will write to it immediately, but for Vec we need set_len
    buffer.set_len(return_length as usize);

    let status = NtQueryInformationProcess(
      handle,
      process_command_line_information,
      buffer.as_mut_ptr() as *mut c_void,
      return_length,
      &mut return_length,
    );

    let _ = CloseHandle(handle);

    if status < 0 {
      return None;
    }

    // buffer starts with UNICODE_STRING structure
    let unicode_string = &*(buffer.as_ptr() as *const UNICODE_STRING);

    if unicode_string.Length == 0 || unicode_string.Buffer.is_null() {
      return Some(String::new());
    }

    // The Buffer pointer in UNICODE_STRING points to the string data *inside* our `buffer` (usually).
    // Or it points to valid memory.
    let slice_len = (unicode_string.Length / 2) as usize;
    let slice = std::slice::from_raw_parts(unicode_string.Buffer.as_ptr(), slice_len);

    Some(String::from_utf16_lossy(slice))
  }
}

// PEB method (more complex, requires more permissions)
// pub fn get_process_cmdline_peb(pid: u32) -> Option<String> {
//   unsafe {
//     let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

//     let mut pbi: PROCESS_BASIC_INFORMATION = std::mem::zeroed();
//     let mut return_length: u32 = 0;
//     let status = NtQueryInformationProcess(
//       handle,
//       0 as _,
//       &mut pbi as *mut _ as *mut c_void,
//       std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
//       &mut return_length,
//     );

//     if status != 0 {
//       // Not STATUS_SUCCESS
//       let _ = CloseHandle(handle);
//       return None;
//     }

//     let peb_base_address = pbi.PebBaseAddress;
//     let mut peb: PEB = std::mem::zeroed();

//     // Read PEB
//     // Using ReadProcessMemory from windows crate
//     let mut bytes_read = 0;
//     if ReadProcessMemory(
//       handle,
//       peb_base_address as _,
//       &mut peb as *mut _ as *mut c_void,
//       std::mem::size_of::<PEB>(),
//       Some(&mut bytes_read),
//     )
//     .is_err()
//     {
//       let _ = CloseHandle(handle);
//       return None;
//     }

//     let process_parameters_ptr = peb.ProcessParameters;
//     let mut process_parameters: RTL_USER_PROCESS_PARAMETERS = std::mem::zeroed();

//     if ReadProcessMemory(
//       handle,
//       process_parameters_ptr as _,
//       &mut process_parameters as *mut _ as *mut c_void,
//       std::mem::size_of::<RTL_USER_PROCESS_PARAMETERS>(),
//       Some(&mut bytes_read),
//     )
//     .is_err()
//     {
//       let _ = CloseHandle(handle);
//       return None;
//     }

//     let cmd_line = process_parameters.CommandLine;
//     if cmd_line.Length == 0 || cmd_line.Buffer.is_null() {
//       let _ = CloseHandle(handle);
//       return Some(String::new()); // Empty cmdline
//     }

//     let mut buffer = vec![0u16; (cmd_line.Length as usize) / 2];
//     if ReadProcessMemory(
//       handle,
//       cmd_line.Buffer as _,
//       buffer.as_mut_ptr() as *mut c_void,
//       buffer.len() * 2,
//       Some(&mut bytes_read),
//     )
//     .is_err()
//     {
//       let _ = CloseHandle(handle);
//       return None;
//     }

//     let _ = CloseHandle(handle);
//     Some(String::from_utf16_lossy(&buffer))
//   }
// }
