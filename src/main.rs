use libc::{self, WIFEXITED, WIFSIGNALED, WUNTRACED, fork, pid_t, waitpid};
use std::{
    env,
    ffi::{CString, c_int},
    fs::{self, ReadDir},
    io::{self, BufRead},
    path::PathBuf,
    process::ExitCode,
};

type BuiltIn = fn(Vec<String>) -> ExitCode;
static BUILT_INS: [(&'static str, BuiltIn); 4] = [
    ("cd", shox_cd),
    ("help", shox_help),
    ("exit", shox_exit),
    ("ls", shox_ls),
];

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
        status = shox_exec(args);
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

fn shox_launch(args: Vec<String>) -> ExitCode {
    let mut code = ExitCode::SUCCESS;
    let c_args: Vec<*const i8> = args
        .iter()
        .map(|x| {
            CString::new(x.as_str())
                .unwrap_or({
                    code = ExitCode::FAILURE;
                    CString::new("").unwrap()
                })
                .as_ptr()
        })
        .collect();

    // Exit if CString converson failed
    if code == ExitCode::FAILURE {
        return code;
    }

    let pid: pid_t;
    let options: c_int = WUNTRACED;

    // WARN: default value of 0 might cause problems
    let status: *mut c_int = &mut 0; // Raw pointer for status

    unsafe {
        pid = fork();
        // Validate pid
        if pid == 0 {
            // TODO: Sort out execvp
            if libc::execvp(c_args[0], c_args.as_ptr()) == -1 {
                eprintln!("{:#?}", io::Error::last_os_error()) // Print os error
            }
            return ExitCode::FAILURE;
        } else if pid < 0 {
            // fork failure
            return ExitCode::FAILURE;
        } else {
            loop {
                waitpid(pid, status, options);
                if WIFEXITED(*status) && WIFSIGNALED(*status) {
                    break;
                }
            }
        }
    };
    code
}

fn shox_exec(args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        return ExitCode::SUCCESS;
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
fn shox_cd(args: Vec<String>) -> ExitCode {
    if args.len() < 2 {
        eprintln!("shox: expected argument for \"cd\"");
    } else {
        let cwd: PathBuf;
        match env::current_dir() {
            Ok(path) => cwd = path,
            Err(e) => {
                eprintln!("{:#?}", e);
                return ExitCode::FAILURE;
            }
        };
        let nwd: PathBuf = cwd.join(&args[1]);
        match env::set_current_dir(nwd) {
            Ok(()) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{:#?}", e);
                return ExitCode::FAILURE;
            }
        }
    }
    return ExitCode::SUCCESS;
}

/// Print list of all available commands
fn shox_help(_args: Vec<String>) -> ExitCode {
    println!("SHOx by Korben Tompkin");
    println!("Available Commands:");
    for (name, _func) in BUILT_INS {
        println!("{}", name);
    }
    ExitCode::SUCCESS
}

/// Exit the program immediately
fn shox_exit(_args: Vec<String>) -> ExitCode {
    std::process::exit(1)
}

/// Primitive ls implementation
fn shox_ls(_args: Vec<String>) -> ExitCode {
    let mut code: ExitCode = ExitCode::SUCCESS;
    let cwd: PathBuf;
    match env::current_dir() {
        Ok(p) => cwd = p,
        Err(e) => {
            eprintln!("{:#?}", e);
            return ExitCode::FAILURE;
        }
    }
    let entries: ReadDir;
    match fs::read_dir(cwd) {
        Ok(x) => entries = x,
        Err(e) => {
            eprintln!("{:#?}", e);
            return ExitCode::FAILURE;
        }
    }
    entries.for_each(|x| match x {
        Ok(entry) => {
            eprint!("{:?}   ", entry.file_name()); // Print file names separated by 3 spaces
        }
        Err(e) => {
            eprintln!("{}", e);
            code = ExitCode::FAILURE;
        }
    });
    eprint!("\n");
    return code;
}
