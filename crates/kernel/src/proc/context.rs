use uefi::cstr16;
// use volatile::{VolatileRef, access::ReadOnly};
use x86_64::{VirtAddr, registers::rflags::RFlags, structures::idt::InterruptStackFrameValue};

use crate::{
    RegistersValue,
    memory::gdt::{get_selector, get_user_selector},
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessContextValue {
    pub regs: RegistersValue,
    pub stack_frame: InterruptStackFrameValue,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProcessContext {
    value: ProcessContextValue,
}

impl ProcessContext {
    #[inline]
    pub fn as_mut(&mut self) -> &mut ProcessContextValue {
        &mut self.value
    }

    #[inline]
    pub fn as_ref(&self) -> &ProcessContextValue {
        &self.value
    }

    #[inline]
    pub fn set_rax(&mut self, value: usize) {
        self.value.regs.rax = value;
    }

    #[inline]
    pub fn save(&mut self, context: &ProcessContext) {
        self.value = *context.as_ref();
    }

    #[inline]
    pub fn restore(&self, context: &mut ProcessContext) {
        *context.as_mut() = self.value;
    }

    pub fn init_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.value.stack_frame.stack_pointer = stack_top;
        self.value.stack_frame.instruction_pointer = entry;
        self.value.stack_frame.cpu_flags =
            RFlags::IOPL_HIGH | RFlags::IOPL_LOW | RFlags::INTERRUPT_FLAG;

        let selector = get_user_selector();

        let mut cs = selector.user_code_selector;
        let mut ds = selector.user_data_selector;

        cs.set_rpl(x86_64::PrivilegeLevel::Ring3);
        ds.set_rpl(x86_64::PrivilegeLevel::Ring3);

        self.value.stack_frame.code_segment = cs;
        self.value.stack_frame.stack_segment = ds;

        trace!("Init stack frame: {:#?}", &self.stack_frame);
    }
}

impl Default for ProcessContextValue {
    fn default() -> Self {
        Self {
            regs: RegistersValue::default(),
            stack_frame: InterruptStackFrameValue::new(
                VirtAddr::new(0x1000),
                get_user_selector().user_code_selector,
                RFlags::empty(),
                VirtAddr::new(0x2000),
                get_user_selector().user_data_selector,
            ),
        }
    }
}

impl core::ops::Deref for ProcessContext {
    type Target = ProcessContextValue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl core::fmt::Debug for ProcessContext {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

impl core::fmt::Debug for ProcessContextValue {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let mut f = f.debug_struct("StackFrame");
        f.field("stack_top", &self.stack_frame.stack_pointer);
        f.field("cpu_flags", &self.stack_frame.cpu_flags);
        f.field("instruction_pointer", &self.stack_frame.instruction_pointer);
        f.field("regs", &self.regs);
        f.finish()
    }
}
