use libc::{self, WIFEXITED, WIFSIGNALED, WUNTRACED, fork, pid_t, waitpid};
use std::{
    env,
    ffi::{CString, c_int},
    fmt::Display,
    fs::{self, ReadDir},
    io::{self, BufRead},
    path::PathBuf,
    process::ExitCode,
};

use crate::ShoxError::{ForkError, IoError};

#[derive(Debug)]
enum ShoxError {
    IoError(io::Error),
    ForkError,
}

impl Display for ShoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShoxError::IoError(io_error) => write!(f, "{}", io_error),
            ShoxError::ForkError => write!(f, "ForkError"),
        }
    }
}

impl From<std::io::Error> for ShoxError {
    fn from(err: std::io::Error) -> Self {
        ShoxError::IoError(err)
    }
}

impl std::error::Error for ShoxError {}

type BuiltIn = fn(Vec<String>) -> Result<(), ShoxError>;
static BUILT_INS: [(&'static str, BuiltIn); 3] =
    [("cd", shox_cd), ("help", shox_help), ("exit", shox_exit)];

fn main() -> ExitCode {
    shox_loop()
}

fn shox_loop() -> ExitCode {
    let mut line: String = "".to_owned();
    let mut status: ExitCode;
    loop {
        eprint!("> ");
        match shox_read_line() {
            Some(s) => {
                line = s;
            }
            None => println!("Failed to read line!"),
        }
        // println!("{}", line.to_string());
        let args = shox_split_line(line.to_string());
        // println!("{:#?}", args);
        status = match shox_exec(args) {
            Ok(_) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
        if status == ExitCode::FAILURE {
            break;
        }
    }
    status
}

fn shox_read_line() -> Option<String> {
    let stdin = io::stdin();
    let line = stdin.lock().lines().next();
    match line {
        Some(Ok(s)) => Some(s),
        Some(Err(e)) => {
            println!("{:#?}", e); // Print the error and continue
            None
        }
        None => None,
    }
}

fn shox_split_line(line: String) -> Vec<String> {
    line.split_whitespace().map(|x| x.to_owned()).collect()
}

fn shox_launch(args: Vec<String>) -> Result<(), ShoxError> {
    // WARN: Will panic if CString conversion fails
    let c_args: Vec<*const i8> = args
        .iter()
        .map(|x| CString::new(x.as_str()).unwrap().as_ptr()) // Ok to panic if CString conversion fails
        .collect();

    let pid: pid_t;
    let options: c_int = WUNTRACED;

    // WARN: default value of 0 might cause problems
    let status: *mut c_int = &mut 0; // Raw pointer for status

    unsafe {
        pid = fork();
        // Validate pid
        if pid == 0 {
            if libc::execvp(c_args[0], c_args.as_ptr()) == -1 {
                eprintln!("{:#?}", io::Error::last_os_error()) // Print os error
            }
            return Err(ForkError);
        } else if pid < 0 {
            // fork failure
            return Err(ForkError);
        } else {
            loop {
                waitpid(pid, status, options);
                if WIFEXITED(*status) && WIFSIGNALED(*status) {
                    break;
                }
            }
        }
    };
    Ok(())
}

fn shox_exec(args: Vec<String>) -> Result<(), ShoxError> {
    if args.is_empty() {
        return Ok(());
    } else {
        for (name, func) in BUILT_INS {
            if args[0] == name {
                return func(args);
            }
        }
    }
    return shox_launch(args);
}

// BUILT IN FUNCTIONS

/// Change directory
fn shox_cd(args: Vec<String>) -> Result<(), ShoxError> {
    if args.len() < 2 {
        eprintln!("shox: expected argument for \"cd\"");
    } else {
        let cwd: PathBuf;
        match env::current_dir() {
            Ok(path) => cwd = path,
            Err(e) => {
                eprintln!("{:#?}", e);
                return Err(IoError(e));
            }
        };
        let nwd: PathBuf = cwd.join(&args[1]);
        env::set_current_dir(nwd)?;
    }
    return Ok(());
}

/// Print list of all available commands
fn shox_help(_args: Vec<String>) -> Result<(), ShoxError> {
    println!("SHOx by Korben Tompkin");
    println!("Available Commands:");
    for (name, _func) in BUILT_INS {
        println!("{}", name);
    }
    Ok(())
}

/// Exit the program immediately
fn shox_exit(_args: Vec<String>) -> Result<(), ShoxError> {
    std::process::exit(1)
}
