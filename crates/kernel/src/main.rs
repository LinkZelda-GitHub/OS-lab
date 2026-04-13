#![no_std]
#![no_main]

use ysos::*;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);
    proc::list_app();

    if let Some(pid) = proc::spawn("hello") {
        log::info!("Started user app 'hello' with PID: {}", pid.0);

        // 使用更安全的轮询等待（由于前面有时钟锁、调试锁影响，
        // 频繁通过 hlt 可能无法接收所有的软中断返回。
        loop {
            if let Some(code) = proc::get_process_exit_code(pid) {
                log::info!("User app 'hello' exited cleanly with code: {}", code);
                break;
            }

            // 每次循环稍微等一下中断
            x86_64::instructions::hlt();
        }
    } else {
        log::error!("Failed to spawn 'hello'.");
    }

    // ysos::wait(spawn_init());
    ysos::shutdown();
}

// pub fn spawn_init() -> proc::ProcessId {
//     // NOTE: you may want to clear the screen before starting the shell
//     // print!("\x1b[1;1H\x1b[2J");

//     proc::list_app();
//     proc::spawn("sh").unwrap()
// }
