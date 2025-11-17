use std::{collections::BTreeMap, env::set_current_dir, path::Path, process};
use nix::{libc::{SIGALRM, SIGCHLD}, sys::{signal::{sigprocmask, SigSet, SigmaskHow}, signalfd::SignalFd, wait::{waitpid, WaitPidFlag}}, unistd::{alarm, Pid}};
use unit::Unit;

const TIMEOUT: u32 = 30;

fn main() {
    if process::id() != 1 {
        panic!("Not running as PID 1, exiting...");
    }
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&SigSet::all()), None).expect("Couldn't block signals");
    set_current_dir(Path::new("/")).expect("Couldn't change directory to /");
    println!("Welcome!");

    let _ = alarm::set(TIMEOUT);
    let signal_fd = SignalFd::new(&SigSet::all()).expect("Couldn't create descriptor for reading signals");

    let mut units = BTreeMap::new();
    unit::load_unit(&mut units, "default.target").expect("failed to load unit");
    unit::traverse_unit(&mut units, "default.target");

    println!("{units:#?}");

    loop {
        match signal_fd.read_signal() {
            Ok(Some(sig)) => handle_signal(sig.ssi_signo),
            _ => {}
        };
    }
}

fn handle_signal(signo: u32) {
    match signo as i32 {
        SIGALRM | SIGCHLD => reap_zombies(),
        _ => {}
    }
}

fn reap_zombies() {
    let _ = waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG));
    let _ = alarm::set(TIMEOUT);
}
