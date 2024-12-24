use std::ffi::{CString};
use std::fs::File;
use std::io::Error;
use std::os::fd::{FromRawFd};
use libc::{c_int, fork, mkfifo, pid_t};

pub enum ForkResult {
    Parent(pid_t),
    Child
}

pub fn safe_fork() -> Result<ForkResult, Error> {
    match unsafe { fork() } {
        -1 => Err(Error::last_os_error()),
        0 => Ok(ForkResult::Child),
        res => Ok(ForkResult::Parent(res)),
    }
}

pub fn new_pipe() -> Result<(File, File), Error> {
    let mut fds: [c_int; 2]  = [0, 0];
    let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if result != 0 {
        return Err(Error::last_os_error());
    }

    Ok((unsafe { File::from_raw_fd(fds[0]) }, unsafe { File::from_raw_fd(fds[1]) }))
}

pub fn new_named_pipe(name: &str) -> Result<String, Error> {
    let full_name = format!("/tmp/jwx_client_{name}");
    let res: c_int = unsafe {
        let cstr = CString::new(full_name.clone())?;
        mkfifo(cstr.as_ptr(), 0o666)
    };

    match res {
        0 => Ok(full_name),
        _ => Err(Error::last_os_error())
    }
}