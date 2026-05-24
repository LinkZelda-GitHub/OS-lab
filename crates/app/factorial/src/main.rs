#![no_std]
#![no_main]

use lib::*;
extern crate lib;

const MOD: u64 = 1000000007;

fn factorial(n: u64) -> u64 {
    if n == 0 {
        // println!("\n--- [System Status When Received Peak Memory Usage] ---");
        // sys_stat();
        1
    } else {
        n * factorial(n - 1) % MOD
    }
}

fn main() -> isize {
    print!("Input n: ");

    let input = lib::stdin().read_line();
    let input = input.trim();

    // parse input as u64
    if let Ok(n) = input.parse::<u64>() {
        if n > 1000000 {
            println!("n must be less than 1000000");
            return 1;
        }

        // println!("\n--- [System Status BEFORE Calculation] ---");
        // sys_stat();

        // println!("\nCalculating factorial...");
        let result = factorial(n);

        // println!("\n--- [System Status AFTER Calculation] ---");
        sys_stat();

        // print result
        println!("The factorial of {} under modulo {} is {}.", n, MOD, result);
        0
    } else {
        println!("Invalid input.");
        1
    }
}

entry!(main);
