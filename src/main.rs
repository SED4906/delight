mod unit;

use std::{env::set_current_dir, mem::zeroed, path::Path, process, ptr::null_mut};

use libc::{alarm, c_uint, sigfillset, sigprocmask, sigset_t, sigwait, waitpid, SIGALRM, SIGCHLD, SIG_BLOCK, WNOHANG};

const TIMEOUT: c_uint = 30;

static mut SIGSET: sigset_t = unsafe { zeroed() };

fn main() {
    if process::id() != 1 {
        panic!("Not running as PID 1, exiting...");
    }
    set_current_dir(Path::new("/")).expect("Couldn't change directory to /");
    println!("Welcome!");
    block_signals();
    let mut units = unit::walk("default.target".into());
    unit::depends_sort(&mut units);
    for unit in units {
        print!("{} ", unit.plain_name());
    }
    loop {
        handle_signals();
    }
}

fn block_signals() {
    unsafe {
        sigfillset(&raw mut SIGSET);
        sigprocmask(SIG_BLOCK, &raw const SIGSET, null_mut());
    }
}

fn handle_signals() {
    unsafe {
        alarm(TIMEOUT);
        let mut signal = 0;
        sigwait(&raw const SIGSET, &raw mut signal);
        match signal {
            SIGCHLD | SIGALRM => {reap_zombies()}
            _ => {}
        }
    }
}

fn reap_zombies() {
    unsafe {
        while waitpid(-1, null_mut(), WNOHANG) > 0 {}
        alarm(TIMEOUT);
    }
}
