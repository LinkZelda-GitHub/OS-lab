#![no_std]
#![no_main]

use core::arch::x86_64::_SIDD_LEAST_SIGNIFICANT;

use lib::*;
extern crate lib;

fn main() -> isize {
    println!("Welcome to YatSenOS Shell!");
    println!("Type 'help' for instructions.");

    // let mut last_input_empty = false;
    loop {
        // if !last_input_empty {
        //     print!("[>] ");
        // } else {
        //     print!("[=] ");
        // }
        print!("[>] ");
        // last_input_empty = false;
        let cmd_line = lib::stdin().read_line();
        let cmd = cmd_line.trim();

        if cmd.is_empty() {
            // last_input_empty = true;
            continue;
        }

        match cmd {
            "help" => {
                println!("YatSenOS Shell Help");
                println!("Commands:");
                println!("  list_app   - List available applications");
                println!("  stat       - List running processes");
                println!("  clear      - Clear the screen");
                println!("  ls [dir]   - List files in a directory");
                println!("  cat <file> - Print the content of a file");
                println!("  help       - Show this help message");
                println!("  <app_name> - Run the specified application");
                println!("Student ID: 24325005");
            }
            "list_app" => {
                sys_list_app();
            }
            "stat" | "ps" => {
                sys_stat();
            }
            "clear" => {
                print!("\x1b[2J\x1b[1;1H");
            }
            "exit" => {
                println!("Goodbye!");
                break;
            }
            cmd if cmd.starts_with("ls") => {
                let mut path = cmd.strip_prefix("ls").unwrap().trim();
                if path.is_empty() {
                    path = "/";
                }
                sys_list_dir(path);
            }
            cmd if cmd.starts_with("cat ") => {
                let path = cmd.strip_prefix("cat ").unwrap().trim();
                if let Some(fd) = sys_open(path) {
                    let mut buf = [0u8; 128];
                    while let Some(len) = sys_read(fd, &mut buf) {
                        if len == 0 {
                            break;
                        }
                        print!("{}", core::str::from_utf8(&buf[..len]).unwrap_or(""));
                    }
                    sys_close(fd);
                    println!("");
                } else {
                    println!("cat: {}: No such file or directory", path);
                }
            }
            _ => {
                let target = alloc::format!("/APP/{}", cmd.to_ascii_uppercase());
                let pid = sys_spawn(&target);
                if pid == 0 {
                    // last_input_empty = true;
                    println!("Unknown command or application: '{}'", cmd);
                } else {
                    let exit_code = sys_wait_pid(pid);
                    println!("Process [{}] exited with code: {}", pid, exit_code);
                }
            }
        }
    }
    0
}

entry!(main);
