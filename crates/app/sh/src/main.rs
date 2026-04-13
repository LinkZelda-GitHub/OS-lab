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
            _ => {
                let pid = sys_spawn(cmd);
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
