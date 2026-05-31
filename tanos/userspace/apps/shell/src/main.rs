//! TanOS Shell
//!
//! Interactive command shell. Receives keypresses from the keyboard driver
//! via IPC, echoes to VGA, and handles builtin commands.

#![no_std]
#![no_main]

#[macro_use]
extern crate libmicro;
extern crate alloc;

use kernel_types::EndpointId;
use libmicro::syscall;
use libmicro::protocols::{endpoints, KeyboardOp, DisplayOp};
use libmicro::{ServerMessage, MessageType};

const MAX_LINE: usize = 256;

struct Shell {
    endpoint: EndpointId,
    display_ep: EndpointId,
    line_buf: [u8; MAX_LINE],
    line_len: usize,
}

impl Shell {
    fn new() -> core::result::Result<Self, ()> {
        // Create our endpoint and register as the shell service
        let endpoint = syscall::create_endpoint().map_err(|_| ())?;

        Ok(Self {
            endpoint,
            display_ep: EndpointId::new_unchecked(endpoints::DISPLAY_SERVICE),
            line_buf: [0u8; MAX_LINE],
            line_len: 0,
        })
    }

    fn display_char(&self, ch: u8) {
        let mut msg = ServerMessage::new(MessageType::Send);
        msg.set_label(DisplayOp::WriteChar as u32);
        msg.set_data(0, ch as u64);
        let _ = syscall::send_message(self.display_ep, &msg.data[0].to_le_bytes());
    }

    fn display_str(&self, s: &str) {
        for b in s.bytes() {
            self.display_char(b);
        }
    }

    fn display_clear(&self) {
        let mut msg = ServerMessage::new(MessageType::Send);
        msg.set_label(DisplayOp::ClearScreen as u32);
        let _ = syscall::send_message(self.display_ep, &msg.data[0].to_le_bytes());
    }

    fn prompt(&self) {
        self.display_str("tanos> ");
    }

    fn handle_key(&mut self, ascii: u8) {
        match ascii {
            b'\n' => {
                self.display_char(b'\n');
                self.execute_line();
                self.line_len = 0;
                self.prompt();
            }
            0x08 => {
                // Backspace
                if self.line_len > 0 {
                    self.line_len -= 1;
                    self.display_char(0x08);
                }
            }
            ch if ch >= 0x20 && ch <= 0x7E => {
                if self.line_len < MAX_LINE {
                    self.line_buf[self.line_len] = ch;
                    self.line_len += 1;
                    self.display_char(ch);
                }
            }
            _ => {}
        }
    }

    fn execute_line(&mut self) {
        let line = match core::str::from_utf8(&self.line_buf[..self.line_len]) {
            Ok(s) => s.trim(),
            Err(_) => {
                self.display_str("invalid input\n");
                return;
            }
        };

        if line.is_empty() {
            return;
        }

        // Split into command + args
        let (cmd, args) = match line.find(' ') {
            Some(i) => (&line[..i], line[i+1..].trim()),
            None => (line, ""),
        };

        match cmd {
            "help" => self.cmd_help(),
            "echo" => self.cmd_echo(args),
            "clear" => self.cmd_clear(),
            "version" => self.cmd_version(),
            "ps" => self.cmd_ps(),
            "mem" => self.cmd_mem(),
            "uptime" => self.cmd_uptime(),
            _ => {
                self.display_str(cmd);
                self.display_str(": command not found\n");
            }
        }
    }

    fn cmd_help(&self) {
        self.display_str("TanOS Shell - Built-in Commands:\n");
        self.display_str("  help     - Show this help\n");
        self.display_str("  echo     - Print arguments\n");
        self.display_str("  clear    - Clear screen\n");
        self.display_str("  version  - Show OS version\n");
        self.display_str("  ps       - List processes\n");
        self.display_str("  mem      - Memory statistics\n");
        self.display_str("  uptime   - System uptime\n");
    }

    fn cmd_echo(&self, args: &str) {
        self.display_str(args);
        self.display_char(b'\n');
    }

    fn cmd_clear(&self) {
        self.display_clear();
    }

    fn cmd_version(&self) {
        self.display_str("TanOS v3.0.0 (x86_64)\n");
        self.display_str("Tanenbaum Microkernel OS\n");
    }

    fn cmd_ps(&self) {
        let pid = syscall::getpid();
        self.display_str("PID  NAME\n");
        self.display_str("  0  kernel\n");
        self.display_str("  1  init\n");
        // Show our own PID
        let mut buf = [0u8; 8];
        let n = fmt_u16(pid.as_u16(), &mut buf);
        self.display_str("  ");
        if let Ok(s) = core::str::from_utf8(&buf[..n]) {
            self.display_str(s);
        }
        self.display_str("  shell\n");
    }

    fn cmd_mem(&self) {
        match syscall::get_stats() {
            Ok(val) => {
                self.display_str("Memory stats: ");
                let mut buf = [0u8; 20];
                let n = fmt_u64(val, &mut buf);
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    self.display_str(s);
                }
                self.display_char(b'\n');
            }
            Err(_) => {
                self.display_str("Could not retrieve memory stats\n");
            }
        }
    }

    fn cmd_uptime(&self) {
        match syscall::get_time() {
            Ok(ms) => {
                let secs = ms / 1000;
                self.display_str("Uptime: ");
                let mut buf = [0u8; 20];
                let n = fmt_u64(secs, &mut buf);
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    self.display_str(s);
                }
                self.display_str("s\n");
            }
            Err(_) => {
                self.display_str("Could not retrieve uptime\n");
            }
        }
    }

    fn run(&mut self) -> ! {
        self.display_str("Welcome to TanOS!\n");
        self.display_str("Type 'help' for available commands.\n\n");
        self.prompt();

        let mut recv_buf = [0u8; 64];
        loop {
            match syscall::receive_message(self.endpoint, &mut recv_buf) {
                Ok(_len) => {
                    // Decode: first 8 bytes = data[0] = ASCII byte
                    let ascii = recv_buf[0];
                    if ascii != 0 {
                        self.handle_key(ascii);
                    }
                }
                Err(_) => {
                    // No message ready, yield
                    let _ = syscall::yield_cpu();
                }
            }
        }
    }
}

/// Format u16 as decimal into buffer. Returns bytes written.
fn fmt_u16(mut val: u16, buf: &mut [u8]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 5];
    let mut i = 0;
    while val > 0 {
        tmp[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    for j in 0..i {
        buf[j] = tmp[i - 1 - j];
    }
    i
}

/// Format u64 as decimal into buffer. Returns bytes written.
fn fmt_u64(mut val: u64, buf: &mut [u8]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        tmp[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    for j in 0..i {
        buf[j] = tmp[i - 1 - j];
    }
    i
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_println!("TanOS shell starting...");

    let pid = syscall::getpid();
    debug_println!("Shell PID: {}", pid.as_u16());

    let mut shell = match Shell::new() {
        Ok(s) => s,
        Err(_) => {
            debug_println!("Failed to initialize shell");
            syscall::exit(-1);
        }
    };

    shell.run();
}
