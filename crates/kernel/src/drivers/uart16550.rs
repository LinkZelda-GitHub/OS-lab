use core::fmt;
use x86_64::instructions::port::Port;
use bitflags::bitflags;

// 串口寄存器偏移量
const DATA: u16 = 0;
const IER:   u16 = 1;  // 中断使能寄存器
const FCR:   u16 = 2;  // FIFO 控制寄存器
const LCR:   u16 = 3;  // 线控寄存器
const MCR:   u16 = 4;  // 调制解调器控制寄存器
const LSR:   u16 = 5;  // 线状态寄存器

bitflags! {
    /// 线控寄存器 (LCR) 的标志位
    pub struct LcrFlags: u8 {
        /// 字长选择位 0 (LSB)
        const WORD_LENGTH_LSB = 1 << 0;
        /// 字长选择位 1 (MSB)
        const WORD_LENGTH_MSB = 1 << 1;
        /// 停止位 (1 表示 2 个停止位，0 表示 1 个停止位)
        const STOP_BITS       = 1 << 2;
        /// 校验使能
        const PARITY_ENABLE   = 1 << 3;
        /// 偶校验选择 (与 PARITY_ENABLE 配合使用）
        const EVEN_PARITY     = 1 << 4;
        /// 粘性校验
        const STICK_PARITY    = 1 << 5;
        /// 中断控制 (Break)
        const BREAK           = 1 << 6;
        /// 除数锁存器访问位 (DLAB)
        const DLAB            = 1 << 7;
    }
}

/// A port-mapped UART 16550 serial interface.
pub struct SerialPort {
    base: u16, // 端口的 base 地址
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { base: port }
    }
    
    /// Initializes the serial port.
    pub fn init(&self) {
        // FIXED: Initialize the serial port
        let base = self.base;

        // 禁用所有中断（IER 寄存器写 0）
        unsafe {
            Port::new(base + IER).write(0x00u8);
        }

        // 设置 DLAB = 1 以访问除数锁存器
        unsafe {
            Port::new(base + LCR).write(LcrFlags::DLAB.bits());
        }

        // 设置波特率除数
        unsafe {
            Port::new(base + DATA).write(0x03u8); // 低字节
            Port::new(base + IER).write(0x00u8);   // 高字节
        }

        // 设置数据格式：8 位数据，无校验，1 位停止位
        unsafe {
            Port::new(base + LCR).write((LcrFlags::WORD_LENGTH_LSB | LcrFlags::WORD_LENGTH_MSB).bits());
        }

        // 启用 FIFO，清除 FIFO，设置 14 字节阈值
        unsafe {
            // Port::new(base + FCR).write(0xC7u8);
            Port::new(base + FCR).write(0x07u8);
        }

        // 先设置 MODEM 控制寄存器为 0x0B，再设为 0x1E 进入回环模式
        unsafe {
            Port::new(base + MCR).write(0x0Bu8);
            Port::new(base + MCR).write(0x1Eu8);
        }

        // 测试芯片：写入 0xAE，检查读入数据是否相同
        unsafe {
            Port::new(base + DATA).write(0xAEu8);
            let test_byte: u8 = Port::new(base + DATA).read();
            if test_byte != 0xAE {
                panic!("Serial port test failed: sent 0xAE, received 0x{:X}", test_byte);
            }
        }

        // 设置正常操作模式：回环关闭，IRQ 启用，OUT1/OUT2 置位
        unsafe {
            Port::new(base + MCR).write(0x0Fu8);
        }

        // 打开所有中断（IER 寄存器写 1）
        unsafe {
            Port::new(base + IER).write(0x01u8);
        }
    }

    /// 判断发送保持寄存器是否为空
    pub fn is_transmit_empty(&mut self) -> bool {
        let base = self.base;

        let lsr: u8 = unsafe { Port::new(base + LSR).read() };
        (lsr & 0x20) != 0
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        // FIXED: Send a byte on the serial port
        let base = self.base;
        // 等待发送保持寄存器为空

        let mut timeout = 0xFFFFF; // 超时计数器，防止死循环

        while !self.is_transmit_empty() {
            core::hint::spin_loop();
            timeout -= 1;
            if timeout == 0 {
                panic!("Timeout while waiting for transmit buffer to be empty");
            }
        }
        // 写入数据寄存器
        unsafe { Port::new(base + DATA).write(data); }
    }

    /// Receives a byte on the serial port no wait.
    pub fn receive(&mut self) -> Option<u8> {
        // FIXED: Receive a byte on the serial port no wait
        let base = self.base;
        let lsr: u8 = unsafe { Port::new(base + LSR).read() };
        if (lsr & 0x01) != 0 {
            // 若数据就绪，则读入并返回
            let data = unsafe { Port::new(base + DATA).read() };
            Some(data)
        } else {
            None
        }
    }
    
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}