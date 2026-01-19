//! Interrupt Descriptor Table (IDT) and exception handlers for JerichoOS
//!
//! Handles CPU exceptions and hardware interrupts

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;
use crate::gdt;
use pic8259::ChainedPics;
use spin::Mutex;

/// PIC interrupt offset
/// We remap PIC interrupts to 32-47 (avoiding 0-31 which are CPU exceptions)
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// Chained PICs (primary and secondary)
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Hardware interrupt indices
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    /// Interrupt Descriptor Table
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // CPU Exception Handlers
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);

        // Double fault handler with separate stack (from GDT/TSS)
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // Hardware interrupt handlers
        idt[InterruptIndex::Timer.as_u8()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// Initialize the IDT and PICs
pub fn init() {
    IDT.load();

    // Initialize PICs
    unsafe {
        PICS.lock().initialize();
    }

    serial_println!("[INFO] IDT loaded, PICs initialized");
}

/// Breakpoint exception handler (#BP)
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("[EXCEPTION] BREAKPOINT\n{:#?}", stack_frame);
}

/// Double fault exception handler (#DF)
/// This has a separate stack to handle stack overflow scenarios
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("[EXCEPTION] DOUBLE FAULT (error: {})\n{:#?}", error_code, stack_frame);
}

/// Page fault exception handler (#PF)
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    serial_println!("[EXCEPTION] PAGE FAULT");
    serial_println!("Accessed Address: {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:#?}", stack_frame);

    loop {
        x86_64::instructions::hlt();
    }
}

/// General protection fault handler (#GP)
extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("[EXCEPTION] GENERAL PROTECTION FAULT (error: {})\n{:#?}", error_code, stack_frame);
}

/// Invalid opcode exception handler (#UD)
extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("[EXCEPTION] INVALID OPCODE\n{:#?}", stack_frame);
}

/// Segment not present handler (#NP)
extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("[EXCEPTION] SEGMENT NOT PRESENT (error: {})\n{:#?}", error_code, stack_frame);
}

/// Stack segment fault handler (#SS)
extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("[EXCEPTION] STACK SEGMENT FAULT (error: {})\n{:#?}", error_code, stack_frame);
}

/// Divide error handler (#DE)
extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("[EXCEPTION] DIVIDE ERROR (division by zero)\n{:#?}", stack_frame);
}

/// Timer tick counter
static TIMER_TICKS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Get timer tick count
pub fn timer_ticks() -> u64 {
    TIMER_TICKS.load(core::sync::atomic::Ordering::Relaxed)
}

/// Timer interrupt handler (IRQ 0)
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment tick counter
    let ticks = TIMER_TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    // Verbose logging only in debug builds (reduces overhead)
    #[cfg(debug_assertions)]
    {
        // Print every 100 ticks (every second at 100 Hz)
        if ticks % 100 == 0 {
            serial_println!("[TIMER] Tick {} ({} s elapsed)", ticks, ticks / 100);
        }
    }

    // Preemptive multitasking: yield to scheduler on every tick
    // This enables time-slice based task switching
    if ticks > 0 {  // Skip first tick (timer setup)
        crate::scheduler::task_yield();
    }

    // Acknowledge interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// Keyboard interrupt handler (IRQ 1)
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    // Read scancode from keyboard
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    serial_println!("[KEYBOARD] Scancode: {:#x}", scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

/// Initialize the PIT (Programmable Interval Timer) and enable interrupts
///
/// Configures the timer to fire at the specified frequency (Hz)
/// Default: 100 Hz (every 10ms)
pub fn init_timer(frequency_hz: u32) {
    use x86_64::instructions::port::Port;

    #[cfg(debug_assertions)]
    serial_println!("[TIMER] Configuring PIT to {} Hz", frequency_hz);

    // PIT base frequency is 1.193182 MHz
    const PIT_FREQUENCY: u32 = 1_193_182;
    let divisor = PIT_FREQUENCY / frequency_hz;

    #[cfg(debug_assertions)]
    serial_println!("[TIMER] PIT divisor: {}", divisor);

    unsafe {
        // Send command byte: channel 0, lobyte/hibyte, rate generator
        let mut cmd_port = Port::<u8>::new(0x43);
        cmd_port.write(0x36);

        // Send divisor (low byte, then high byte)
        let mut data_port = Port::<u8>::new(0x40);
        data_port.write((divisor & 0xFF) as u8);
        data_port.write(((divisor >> 8) & 0xFF) as u8);
    }

    serial_println!("[TIMER] PIT configured, enabling interrupts");

    // Enable interrupts globally
    x86_64::instructions::interrupts::enable();

    serial_println!("[TIMER] Interrupts enabled");
}

/// Test breakpoint exception
#[test_case]
fn test_breakpoint_exception() {
    serial_print!("test_breakpoint_exception...");
    // Invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    serial_println!("[ok]");
}
