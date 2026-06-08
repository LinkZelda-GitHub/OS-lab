//! PS/2 Keyboard Controller Driver
//!
//! Handles communication with the PS/2 keyboard controller at ports 0x60 (data)
//! and 0x64 (status/command). Converts scancodes to key events using
//! `pc-keyboard`.

use pc_keyboard::{DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};
use x86_64::instructions::port::Port;

use crate::drivers::{input, vga_text};

// PS/2 Controller ports
const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const PS2_COMMAND_PORT: u16 = 0x64;

// Status register bits
const PS2_STATUS_OUTPUT_FULL: u8 = 0x01;
const PS2_STATUS_INPUT_FULL: u8 = 0x02;

/// PS/2 Keyboard driver
pub struct Ps2Keyboard {
    data_port: Port<u8>,
    status_port: Port<u8>,
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
}

// SAFETY: PS/2 keyboard IO ports are safe to access from any CPU core
unsafe impl Send for Ps2Keyboard {}
unsafe impl Sync for Ps2Keyboard {}

impl Ps2Keyboard {
    pub fn new() -> Self {
        Self {
            data_port: Port::new(PS2_DATA_PORT),
            status_port: Port::new(PS2_STATUS_PORT),
            keyboard: Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore),
        }
    }

    /// Initialize the PS/2 keyboard
    pub fn init(&mut self) {
        // Wait for the controller to be ready
        self.wait_write();

        // Enable scanning
        unsafe {
            // Send command 0xF4 (enable scanning) to the keyboard
            self.wait_write();
            self.data_port.write(0xF4);

            // Wait for ACK
            let ack = self.read_data();
            if ack != Some(0xFA) {
                warn!(
                    "Keyboard did not ACK enable command, got: {:02x}",
                    ack.unwrap_or(0)
                );
            }
        }

        // Clear any pending data
        self.flush();

        info!("PS/2 Keyboard Initialized.");
    }

    /// Wait until the controller is ready to accept a write
    fn wait_write(&mut self) {
        let mut timeout = 100_000;
        unsafe {
            while (self.status_port.read() & PS2_STATUS_INPUT_FULL) != 0 {
                timeout -= 1;
                if timeout == 0 {
                    warn!("PS/2 controller write timeout");
                    return;
                }
                core::hint::spin_loop();
            }
        }
    }

    /// Wait until the controller has data available
    fn wait_read(&mut self) -> bool {
        let mut timeout = 100_000;
        unsafe {
            loop {
                let status = self.status_port.read();
                if (status & PS2_STATUS_OUTPUT_FULL) != 0 {
                    return true;
                }
                timeout -= 1;
                if timeout == 0 {
                    return false;
                }
                core::hint::spin_loop();
            }
        }
    }

    /// Read a byte from the data port
    fn read_data(&mut self) -> Option<u8> {
        if self.wait_read() {
            unsafe { Some(self.data_port.read()) }
        } else {
            None
        }
    }

    /// Flush any pending data
    fn flush(&mut self) {
        while self.wait_read() {
            unsafe {
                self.data_port.read();
            }
        }
    }

    /// Process an incoming scancode byte and push the resulting character to
    /// the input buffer
    pub fn handle_scancode(&mut self, scancode: u8) {
        if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
            if let Some(decoded) = self.keyboard.process_keyevent(key_event) {
                match decoded {
                    DecodedKey::Unicode(ch) => {
                        // Push the character to the input buffer
                        // For newline/enter, push '\n'; for other chars, push as-is
                        if ch == '\r' {
                            input::push_key(b'\n');
                        } else {
                            // We need to encode the unicode char to UTF-8 and push bytes
                            // For ASCII characters, this is just the byte
                            if ch.is_ascii() {
                                input::push_key(ch as u8);
                            }
                        }
                    }
                    DecodedKey::RawKey(key) => {
                        match key {
                            KeyCode::Backspace | KeyCode::Delete => {
                                input::push_key(0x08); // Backspace
                            }
                            KeyCode::ArrowUp => {
                                // Scroll the VGA console up (view older content)
                                vga_text::scroll_console_up();
                            }
                            KeyCode::ArrowDown => {
                                // Scroll the VGA console down (view newer content)
                                vga_text::scroll_console_down();
                            }
                            KeyCode::PageUp => {
                                // Page-up: scroll by half a screen
                                vga_text::scroll_console_up_pages(1);
                            }
                            KeyCode::PageDown => {
                                // Page-down: scroll by half a screen
                                vga_text::scroll_console_down_pages(1);
                            }
                            _ => {
                                // Other keys ignored for now
                            }
                        }
                    }
                }
            }
        }
    }
}

// Global PS/2 keyboard instance
once_mutex!(pub PS2_KEYBOARD: Ps2Keyboard);
guard_access_fn!(pub get_ps2_keyboard(PS2_KEYBOARD: Ps2Keyboard));

/// Initialize the keyboard driver
pub fn init() {
    let mut kb = Ps2Keyboard::new();
    kb.init();
    init_PS2_KEYBOARD(kb);
}
