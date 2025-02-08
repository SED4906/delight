mod unit;

use std::{collections::BTreeSet, env::set_current_dir, mem::zeroed, path::Path, process, ptr::null_mut};

use libc::{alarm, c_uint, sigfillset, sigprocmask, sigset_t, sigwait, waitpid, SIGALRM, SIGCHLD, SIG_BLOCK, WNOHANG};
use unit::load_unit;

const TIMEOUT: c_uint = 30;

static mut SIGSET: sigset_t = unsafe { zeroed() };

fn main() {
    if process::id() != 1 {
        panic!("Not running as PID 1, exiting...");
    }
    set_current_dir(Path::new("/")).expect("Couldn't change directory to /");
    println!("Welcome!");
    let _ = process::Command::new("mount").args(&["-t","tmpfs","-o","rw,nosuid,relatime,size=50%,nr_inodes=1m,inode64","tmpfs", "/tmp"]).spawn();
    let _ = process::Command::new("mount").args(&["-t","tmpfs","-o","rw,nosuid,relatime,mode=755,inode64","tmpfs", "/run"]).spawn();
    let _ = process::Command::new("mount").args(&["-t","proc","-o","rw,nosuid,nodev,noexec,relatime","proc", "/proc"]).spawn();
    let _ = process::Command::new("mount").args(&["-t","devtmpfs","-o","rw,nosuid,relatime,size=50%,mode=755,nr_inodes=2m,inode64","dev", "/dev"]).spawn();
    let _ = process::Command::new("mount").args(&["-t","sysfs","-o","rw,nosuid,nodev,noexec,relatime","sys", "/sys"]).spawn();
    let mut active_units = BTreeSet::new();
    if load_unit("default.target", &mut active_units, false).is_err() {
        println!("Something in default.target failed that bubbled up to it");
        println!("...Oh well...");
    }
    println!("Attempting to start getty...");
    if load_unit("getty@tty1.service", &mut active_units, false).is_err() {
        println!("Couldn't start getty@tty1.service");
        println!("...Oh well...");
    }
    unsafe {
        block_signals();
        loop {
            handle_signals();
        }
    }
}

unsafe fn block_signals() {
    sigfillset(&raw mut SIGSET);
    sigprocmask(SIG_BLOCK, &raw const SIGSET, null_mut());
}

unsafe fn handle_signals() {
    alarm(TIMEOUT);
    let mut signal = 0;
    sigwait(&raw const SIGSET, &raw mut signal);
    match signal {
        SIGCHLD | SIGALRM => {reap_zombies()}
        _ => {}
    }
}

unsafe fn reap_zombies() {
    while waitpid(-1, null_mut(), WNOHANG) > 0 {}
    alarm(TIMEOUT);
}
