// use libc::WNOWAIT;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        // TODO: implement me!
        let mut cmd = Command::new(target);
        cmd.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        let child = cmd.spawn().ok()?;
        
        match waitpid(nix::unistd::Pid::from_raw(child.id() as i32), Some(WaitPidFlag::WUNTRACED)).ok()? {
            WaitStatus::Stopped(_, signal) => {
                if signal != Signal::SIGTRAP {
                    println!("WaitStatus::Stopped : Not signaled by SIGTRAP!");
                    return None;
                }
            },
            _ => {
                println!("Other Status!");
                return None;
            },
        }
        // println!("Check signal SIGTRAP succeed!");
        let res = Inferior {child};
        Some(res)
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_execute(&self) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), None)?;
        Ok(self.wait(None)?)
    }

    pub fn kill(&mut self) {
        // let res = self.wait(Some(WaitPidFlag::WNOWAIT));
        // if !res.is_ok() {
        //     println!("wait error!");
        //     return;
        // }
        // match res.unwrap() {
        //     Status::Stopped(_, _) => {
        if self.child.kill().is_ok() {
            self.wait(None).ok();
            println!("Killing running inferior (pid {})", self.pid());
        }
    }
}
