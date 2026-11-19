use libc::{self, WIFEXITED, WIFSIGNALED, WUNTRACED, fork, pid_t, waitpid};
use std::{
    env,
    ffi::{CString, c_int},
    fmt::Display,
    io::{self, BufRead, Write},
    path::PathBuf,
    str::Utf8Error,
};

use crate::ShoxError::{ForkError, IoError};

#[derive(Debug)]
enum ShoxError {
    IoError(io::Error),
    ForkError,
    ParseError(Utf8Error),
}

impl Display for ShoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShoxError::IoError(io_error) => write!(f, "{}", io_error),
            ShoxError::ForkError => write!(f, "ForkError"),
            ShoxError::ParseError(utf8_error) => write!(f, "{}", utf8_error),
        }
    }
}

impl From<std::io::Error> for ShoxError {
    fn from(err: std::io::Error) -> Self {
        ShoxError::IoError(err)
    }
}

impl From<Utf8Error> for ShoxError {
    fn from(err: Utf8Error) -> Self {
        ShoxError::ParseError(err)
    }
}

impl std::error::Error for ShoxError {}

type BuiltIn = fn(Vec<String>) -> Result<(), ShoxError>;
static BUILT_INS: [(&str, BuiltIn); 4] = [
    ("cd", shox_cd),
    ("help", shox_help),
    ("exit", shox_exit),
    ("echo", shox_echo),
];

fn main() {
    shox_loop()
}

fn shox_loop() {
    let mut line: String = "".to_owned();
    loop {
        print!("> ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("{}", e);
            continue;
        }
        match shox_read_line() {
            Some(s) => {
                line = s;
            }
            None => println!("Failed to read line!"),
        }
        // println!("{}", line.to_string());
        let args = shox_split_line(line.to_string());
        // println!("{:#?}", args);
        match shox_exec(args) {
            Ok(_) => (),
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn shox_read_line() -> Option<String> {
    let stdin = io::stdin();
    let line = stdin.lock().lines().next();
    match line {
        Some(Ok(s)) => Some(s),
        Some(Err(e)) => {
            eprintln!("{}", e); // Print the error and continue
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
    let c_args: Vec<CString> = args
        .iter()
        .map(|x| CString::new(x.as_str()).unwrap()) // Ok to panic if CString conversion fails
        .collect();

    let argv: Vec<*const i8> = c_args.iter().map(|x| x.as_ptr()).collect();

    let pid: pid_t;
    let options: c_int = WUNTRACED;

    // WARN: default value of 0 might cause problems
    let status: *mut c_int = &mut 0; // Raw pointer for status

    unsafe {
        pid = fork();
        // Validate pid
        if pid == 0 {
            // pid == 0 => child
            if libc::execvp(c_args[0].as_ptr(), argv.as_ptr()) == -1 {
                // Command not found
                eprintln!("SHOx: Command not found: \"{}\"", c_args[0].to_str()?); // Print user facing error message
                libc::_exit(127); // Command not found
            }
            libc::_exit(0) // Success
        } else if pid < 0 {
            // fork failure
            return Err(ForkError);
        } else {
            // Wait on child process to finish
            loop {
                if waitpid(pid, status, options) == -1 {
                    return Err(IoError(io::Error::last_os_error()));
                }
                if WIFEXITED(*status) || WIFSIGNALED(*status) {
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
    shox_launch(args)
}

// BUILT IN FUNCTIONS

/// Change directory
fn shox_cd(args: Vec<String>) -> Result<(), ShoxError> {
    if args.len() < 2 {
        eprintln!("shox: expected argument for \"cd\"");
    } else {
        let cwd: PathBuf = match env::current_dir() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("{}", e);
                return Err(IoError(e));
            }
        };
        let nwd: PathBuf = cwd.join(&args[1]);
        env::set_current_dir(nwd)?;
    }
    Ok(())
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
    std::process::exit(0)
}

/// Print args to screen
fn shox_echo(args: Vec<String>) -> Result<(), ShoxError> {
    println!("{}", args[1..].join(" "));
    Ok(())
}
