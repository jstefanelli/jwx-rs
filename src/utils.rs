use libc::{fork, pid_t};

pub enum ForkResult {
    Parent(pid_t),
    Child
}

pub fn safe_fork() -> Result<ForkResult, i32> {
    match unsafe { fork() } {
        -1 => Err(-1),
        0 => Ok(ForkResult::Child),
        res => Ok(ForkResult::Parent(res)),
    }
}