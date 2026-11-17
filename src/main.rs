use libc::{self, fork, pid_t};
use std::{
    ffi::{CStr, CString},
    io::{self, BufRead},
    process::ExitCode,
};

fn main() -> ExitCode {
    shox_loop()
}

fn shox_loop() -> ExitCode {
    let mut line: String;
    let mut status: ExitCode;
    loop {
        print!("> ");
        loop {
            match shox_read_line() {
                Some(s) => {
                    line = s;
                    break;
                }
                None => println!("Failed to read line!"),
            }
        }
        println!("{}", line);
        let tokens = shox_split_line(line);
        println!("{:#?}", tokens);
        status = shox_exec(tokens);
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

fn shox_exec(args: Vec<String>) -> ExitCode {
    let mut code = ExitCode::SUCCESS;
    let c_args: Vec<CString> = args
        .iter()
        .map(|x| {
            CString::new(x.as_str()).unwrap_or({
                code = ExitCode::FAILURE;
                CString::new("").unwrap()
            })
        })
        .collect();
    unsafe {
        let pid: pid_t = fork();
    }
    code
}
