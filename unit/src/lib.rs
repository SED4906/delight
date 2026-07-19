mod activate;
mod load;
mod parser;
mod state;
mod traverse;

const SYSTEM_UNIT_PATHS: &[&str] = &[
    "/etc/systemd/system/",
    "/usr/local/lib/systemd/system/",
    "/usr/lib/systemd/system/",
];

use std::{ffi::OsStr, os::unix::process::CommandExt, process::Command};

pub use activate::activate_unit;
pub use load::load_unit;
pub use traverse::traverse_unit;

#[derive(Clone, Debug)]
pub struct Unit {
    section: Section,
    // [Unit]
    requires: Vec<String>,
    wants: Vec<String>,
    after: Vec<String>,
    before: Vec<String>,
    // [Install]
    alias: Vec<String>,
    wanted_by: Vec<String>,
    required_by: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Section {
    Service {
        exec: Exec,
        exec_start: Vec<String>,
        exec_stop: Vec<String>,
    },
    Mount {
        exec: Exec,
        what: String,
        r#where: String,
        r#type: String,
        options: Vec<String>,
        sloppy_options: bool,
    },
    Swap {
        exec: Exec,
        what: String,
        priority: Option<String>,
        options: Vec<String>,
    },
    Socket {
        exec: Exec,
        service: Option<String>,
    },
    Target,
    Device,
    Automount {
        r#where: String,
        extra_options: Vec<String>,
    },
    Timer {
        unit: Option<String>,
    },
    Path {
        unit: Option<String>,
    },
    Slice,
    Scope,
}

#[derive(Clone, Debug)]
pub enum UnitKind {
    Service,
    Mount,
    Swap,
    Socket,
    Target,
    Device,
    Automount,
    Timer,
    Path,
    Slice,
    Scope,
}

#[derive(Clone, Debug)]
pub struct Exec {
    exec_search_path: Option<Vec<String>>,
    working_directory: Option<String>,
    user: Option<String>,
    group: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub enum CmdPrivileges {
    #[default]
    Default,
    OnlyUserAndGroups,
    Full,
}

#[derive(Clone, Debug)]
pub struct Cmd {
    replace_arg0: bool,
    ignore_exit_code: bool,
    no_substitution: bool,
    privileges: CmdPrivileges,
    invoke_shell: bool,
    command: String,
}

impl Exec {
    pub fn simple(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        if let Some(ref exec_search_path) = self.exec_search_path {
            command.env("PATH", exec_search_path.join(":"));
        }
        if let Some(ref working_directory) = self.working_directory {
            command.current_dir(working_directory);
        }
        if let Some(ref user) = self.user {
            if let Some(uid) = user.parse::<u32>().ok() {
                command.uid(uid);
            } else {
                println!("user names unimplemented");
            }
        }
        if let Some(ref group) = self.group {
            if let Some(gid) = group.parse::<u32>().ok() {
                command.gid(gid);
            } else {
                println!("group names unimplemented");
            }
        }
        command
    }

    pub fn prog(&self, cmd: Cmd) -> Command {
        let mut args = cmd.command.split_ascii_whitespace();
        let mut command = Command::new(args.next().unwrap_or("false"));
        if cmd.replace_arg0 {
            command.arg0(args.next().unwrap_or("nonexisty"));
        }
        command.args(args);
        if let Some(ref exec_search_path) = self.exec_search_path {
            command.env("PATH", exec_search_path.join(":"));
        }
        if let Some(ref working_directory) = self.working_directory {
            command.current_dir(working_directory);
        }
        if let Some(ref user) = self.user {
            if let Some(uid) = user.parse::<u32>().ok() {
                command.uid(uid);
            } else {
                println!("user names unimplemented");
            }
        }
        if let Some(ref group) = self.group {
            if let Some(gid) = group.parse::<u32>().ok() {
                command.gid(gid);
            } else {
                println!("group names unimplemented");
            }
        }
        command
    }
}

impl Cmd {
    fn new(line: String) -> Cmd {
        let mut replace_arg0 = false;
        let mut ignore_exit_code = false;
        let mut no_substitution = false;
        let mut privileges = CmdPrivileges::Default;
        let mut invoke_shell = false;
        let mut command = line.as_str();
        while command.starts_with(['@', '-', ':', '+', '!', '|']) {
            let prefix;
            (prefix, command) = command.split_at(1);
            match prefix {
                "@" => replace_arg0 = true,
                "-" => ignore_exit_code = true,
                ":" => no_substitution = true,
                "+" => privileges = CmdPrivileges::Full,
                "!" => privileges = CmdPrivileges::OnlyUserAndGroups,
                "|" => invoke_shell = true,
                _ => unreachable!(),
            }
        }
        Cmd {
            replace_arg0,
            ignore_exit_code,
            no_substitution,
            privileges,
            invoke_shell,
            command: command.into(),
        }
    }
}
