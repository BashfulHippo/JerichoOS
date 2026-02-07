// round robin scheduler
// yeah it's not the most efficient, could use a better queue structure

use crate::task::{Task, TaskId, TaskList, TaskState, TaskContext};
use alloc::collections::VecDeque;
use spin::Mutex;

/// Global scheduler instance
pub static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

/// Round-robin task scheduler
pub struct Scheduler {
    /// All tasks in the system
    tasks: TaskList,

    /// Currently running task
    current_task: Option<TaskId>,

    /// Queue of ready tasks
    ready_queue: VecDeque<TaskId>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Scheduler {
            tasks: TaskList::new(),
            current_task: None,
            ready_queue: VecDeque::new(),
        }
    }

    /// Add a task to the scheduler
    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id();
        self.tasks.add(task);
        self.ready_queue.push_back(id);
        serial_println!("[SCHED] Added task {} to scheduler", id.value());
        id
    }

    /// Get current running task ID
    pub fn current_task(&self) -> Option<TaskId> {
        self.current_task
    }

    /// Get task count
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get reference to a task
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get mutable reference to a task
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Schedule next task (round-robin)
    ///
    /// Optimized for performance - minimal logging in hot path
    pub fn schedule(&mut self) -> Option<TaskId> {
        // Get next ready task from queue
        if let Some(next_id) = self.ready_queue.pop_front() {
            // Mark previous task as ready (if any)
            if let Some(current_id) = self.current_task {
                if let Some(current) = self.tasks.get_mut(current_id) {
                    if current.state() == TaskState::Running {
                        current.set_state(TaskState::Ready);
                    }
                }
            }

            // Mark new task as running
            if let Some(next) = self.tasks.get_mut(next_id) {
                if next.state() == TaskState::Ready {
                    next.set_state(TaskState::Running);
                    self.current_task = Some(next_id);

                    // Re-add to ready queue for next round
                    self.ready_queue.push_back(next_id);

                    // Verbose logging only in debug builds
                    #[cfg(debug_assertions)]
                    serial_println!("[SCHED] Scheduled task {} ({})",
                        next_id.value(), next.name());

                    return Some(next_id);
                }
            }
        }

        None
    }

    /// Yield CPU to next task (cooperative multitasking)
    pub fn yield_cpu(&mut self) {
        if let Some(next_id) = self.schedule() {
            // Context switch will happen in assembly
            // For now, just update scheduler state
            serial_println!("[SCHED] Yielding to task {}", next_id.value());
        }
    }

    /// Block current task (for IPC wait)
    pub fn block_current(&mut self) {
        if let Some(current_id) = self.current_task {
            if let Some(task) = self.tasks.get_mut(current_id) {
                task.set_state(TaskState::Blocked);
                serial_println!("[SCHED] Blocked task {}", current_id.value());
            }

            // Remove from ready queue
            self.ready_queue.retain(|&id| id != current_id);

            // Schedule next task
            self.schedule();
        }
    }

    /// Unblock a task (for IPC wake-up)
    pub fn unblock_task(&mut self, task_id: TaskId) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            if task.state() == TaskState::Blocked {
                task.set_state(TaskState::Ready);
                self.ready_queue.push_back(task_id);
                serial_println!("[SCHED] Unblocked task {}", task_id.value());
            }
        }
    }

    /// Terminate current task
    pub fn terminate_current(&mut self) {
        if let Some(current_id) = self.current_task {
            if let Some(task) = self.tasks.get_mut(current_id) {
                task.set_state(TaskState::Terminated);
                serial_println!("[SCHED] Terminated task {}", current_id.value());
            }

            // Remove from ready queue
            self.ready_queue.retain(|&id| id != current_id);

            self.current_task = None;

            // Schedule next task
            self.schedule();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the scheduler
pub fn init() {
    *SCHEDULER.lock() = Some(Scheduler::new());
    serial_println!("[SCHED] Scheduler initialized");
}

/// Get a snapshot of the current task's CSpace
///
/// Returns a cloned CSpace for the currently running task.
/// This allows capability checks without holding the scheduler lock.
///
/// # Assumptions
/// - INTERRUPTS: May be enabled (brief lock acquisition)
/// - LOCKS: No locks held by caller
/// - TRUST: Kernel-only (returns task's security context)
///
/// # Note
/// The returned CSpace is a snapshot. If capabilities are revoked
/// after this call, the snapshot will not reflect those changes.
pub fn current_task_cspace() -> Option<crate::capability::CSpace> {
    let guard = SCHEDULER.lock();
    let scheduler = guard.as_ref()?;
    let current_id = scheduler.current_task()?;
    let task = scheduler.get_task(current_id)?;
    Some(task.cspace().clone())
}

/// Get the current task's ID
///
/// # Assumptions
/// - INTERRUPTS: May be enabled (brief lock acquisition)
/// - LOCKS: No locks held by caller
pub fn current_task_id() -> Option<TaskId> {
    SCHEDULER.lock().as_ref()?.current_task()
}

/// Context switch between tasks
///
/// Saves current task's registers to old_context,
/// Restores new task's registers from new_context
///
/// # Safety
/// This function is unsafe because it manipulates CPU state directly
#[inline(never)]
pub unsafe fn switch_context(old_context: &mut TaskContext, new_context: &TaskContext) {
    core::arch::asm!(
        // Save old task's context
        // rdi = pointer to old_context, rsi = pointer to new_context

        // Save stack pointers
        "mov [rdi + 0], rsp",           // offset 0: rsp
        "mov [rdi + 8], rbp",           // offset 8: rbp

        // Save general purpose registers
        "mov [rdi + 16], rax",          // offset 16: rax
        "mov [rdi + 24], rbx",          // offset 24: rbx
        "mov [rdi + 32], rcx",          // offset 32: rcx
        "mov [rdi + 40], rdx",          // offset 40: rdx
        "mov [rdi + 48], rsi",          // offset 48: rsi (save before we lose it!)
        "mov [rdi + 56], rdi",          // offset 56: rdi (save pointer itself)
        "mov [rdi + 64], r8",           // offset 64: r8
        "mov [rdi + 72], r9",           // offset 72: r9
        "mov [rdi + 80], r10",          // offset 80: r10
        "mov [rdi + 88], r11",          // offset 88: r11
        "mov [rdi + 96], r12",          // offset 96: r12
        "mov [rdi + 104], r13",         // offset 104: r13
        "mov [rdi + 112], r14",         // offset 112: r14
        "mov [rdi + 120], r15",         // offset 120: r15

        // Save flags
        "pushfq",                       // Push RFLAGS onto stack
        "pop rax",                      // Pop into RAX
        "mov [rdi + 128], rax",         // offset 128: rflags

        // Save return address (RIP) - this is where we'll resume
        "lea rax, [rip + 2f]",          // Load address of label 2 (forward)
        "mov [rdi + 136], rax",         // offset 136: rip

        // Now restore new task's context
        // rsi still points to new_context

        // Restore stack pointers
        "mov rsp, [rsi + 0]",           // offset 0: rsp
        "mov rbp, [rsi + 8]",           // offset 8: rbp

        // Restore general purpose registers
        "mov rax, [rsi + 16]",          // offset 16: rax
        "mov rbx, [rsi + 24]",          // offset 24: rbx
        "mov rcx, [rsi + 32]",          // offset 32: rcx
        "mov rdx, [rsi + 40]",          // offset 40: rdx
        "mov r8, [rsi + 64]",           // offset 64: r8
        "mov r9, [rsi + 72]",           // offset 72: r9
        "mov r10, [rsi + 80]",          // offset 80: r10
        "mov r11, [rsi + 88]",          // offset 88: r11
        "mov r12, [rsi + 96]",          // offset 96: r12
        "mov r13, [rsi + 104]",         // offset 104: r13
        "mov r14, [rsi + 112]",         // offset 112: r14
        "mov r15, [rsi + 120]",         // offset 120: r15

        // Restore flags
        "mov rdi, [rsi + 128]",         // offset 128: rflags (use rdi as temp)
        "push rdi",                     // Push onto stack
        "popfq",                        // Pop into RFLAGS

        // Restore rdi and rsi last (we need them for memory access)
        "mov rdi, [rsi + 56]",          // offset 56: rdi
        "mov rax, [rsi + 48]",          // offset 48: rsi (use rax as temp)

        // Jump to new task's RIP
        "push qword ptr [rsi + 136]",   // Push new RIP onto stack
        "mov rsi, rax",                 // Restore rsi from temp
        "ret",                          // Pop RIP and jump

        // Label for resumption point (we return here when switched back to)
        "2:",

        in("rdi") old_context as *mut TaskContext,
        in("rsi") new_context as *const TaskContext,
        clobber_abi("C"),
    );
}

/// Task entry wrapper
///
/// This function is called when a task first starts executing.
/// The task's actual entry point is stored in RDI register.
///
/// # Safety
/// This function never returns. It either runs the task forever or terminates it.
#[unsafe(naked)]
pub extern "C" fn task_entry_wrapper() -> ! {
    core::arch::naked_asm!(
        // RDI contains the entry point address (set up by Task::new)
        // Call the task's entry point
        "call rdi",

        // If we reach here, task returned (shouldn't happen for fn() -> !)
        // Terminate the task
        "call {terminate_task}",

        // Should never reach here
        "2:",
        "hlt",
        "jmp 2b",

        terminate_task = sym terminate_current_task,
    )
}

/// Terminate the current task
///
/// Called by task_entry_wrapper if a task unexpectedly returns
extern "C" fn terminate_current_task() -> ! {
    serial_println!("[SCHED] Task returned unexpectedly, terminating...");

    // Lock scheduler and terminate current task
    if let Some(scheduler) = SCHEDULER.lock().as_mut() {
        scheduler.terminate_current();
    }

    // Halt forever (scheduler should have switched to another task)
    loop {
        x86_64::instructions::hlt();
    }
}

/// Yield CPU to next task (cooperative multitasking)
///
/// This function saves the current task's context and switches to the next ready task.
///
/// # Interrupt Safety
/// Interrupts are disabled for the duration of scheduling and context switch to prevent:
/// 1. Deadlock from nested task_yield() via timer interrupt
/// 2. Task list mutation between pointer extraction and use
///
/// The switched-to task will have its own interrupt state restored from its saved RFLAGS.
/// When this task resumes, interrupts are re-enabled if they were enabled on entry.
pub fn task_yield() {
    use x86_64::instructions::interrupts;

    // === PHASE 1: Disable interrupts ===
    // Record current state to restore later (handles nested interrupt contexts)
    let interrupts_enabled = interrupts::are_enabled();
    interrupts::disable();

    // === PHASE 2: Schedule under lock (interrupts disabled) ===
    let switch_info: Option<(*mut TaskContext, *const TaskContext)> = {
        let mut guard = SCHEDULER.lock();
        let scheduler = match guard.as_mut() {
            Some(s) => s,
            None => {
                // Scheduler not initialized - restore and return
                if interrupts_enabled {
                    interrupts::enable();
                }
                return;
            }
        };

        // Get current task
        let old_id = match scheduler.current_task() {
            Some(id) => id,
            None => {
                if interrupts_enabled {
                    interrupts::enable();
                }
                return;
            }
        };

        // Schedule next task
        let new_id = match scheduler.schedule() {
            Some(id) => id,
            None => {
                if interrupts_enabled {
                    interrupts::enable();
                }
                return;
            }
        };

        // Same task - no switch needed
        if old_id == new_id {
            if interrupts_enabled {
                interrupts::enable();
            }
            return;
        }

        // Extract context pointers while holding lock
        // SAFETY: Pointers are valid because:
        // 1. Lock is held, preventing concurrent mutation
        // 2. Interrupts disabled, preventing same-core preemption
        let old_ctx_ptr = scheduler
            .get_task_mut(old_id)
            .unwrap()
            .context_mut() as *mut TaskContext;

        let new_ctx_ptr = scheduler
            .get_task(new_id)
            .unwrap()
            .context() as *const TaskContext;

        #[cfg(debug_assertions)]
        serial_println!("[SCHED] Switching from task {} to task {}",
            old_id.value(), new_id.value());

        Some((old_ctx_ptr, new_ctx_ptr))
    }; // Lock released here

    // === PHASE 3: Context switch (interrupts still disabled) ===
    // SAFETY: Pointers remain valid because:
    // - Interrupts disabled: no timer, no nested task_yield
    // - Single-core: no concurrent execution possible
    // - Lock released: OK because nothing can run to mutate task list
    if let Some((old_ctx_ptr, new_ctx_ptr)) = switch_info {
        unsafe {
            switch_context(&mut *old_ctx_ptr, &*new_ctx_ptr);
        }
        // === PHASE 4: Resumed after being switched back ===
        // We arrive here when another task switches back to us.
        // Our saved RFLAGS (with IF=0) was restored, so interrupts are still disabled.
    }

    // === PHASE 5: Restore interrupt state ===
    // Re-enable only if they were enabled when we entered
    if interrupts_enabled {
        interrupts::enable();
    }
}
