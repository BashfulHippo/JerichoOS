//! Task management for JerichoOS
//!
//! Provides task/thread abstraction for multitasking

use crate::capability::CSpace;
use alloc::boxed::Box;
use alloc::vec::Vec;
use x86_64::VirtAddr;

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new(id: u64) -> Self {
        TaskId(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Task execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run, waiting for CPU
    Ready,
    /// Currently running on CPU
    Running,
    /// Blocked waiting for IPC or event
    Blocked,
    /// Terminated, can be cleaned up
    Terminated,
}

/// Task priority (for future priority scheduling)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Realtime = 3,
}

/// Saved CPU context for task switching
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    // Stack and base pointers
    pub rsp: u64,
    pub rbp: u64,

    // General purpose registers
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // CPU flags and instruction pointer
    pub rflags: u64,
    pub rip: u64,
}

impl TaskContext {
    /// Create a new empty context
    pub fn new() -> Self {
        TaskContext {
            rsp: 0,
            rbp: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x200, // Interrupt enable flag
            rip: 0,
        }
    }
}

impl Default for TaskContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Task stack size (64 KB)
const TASK_STACK_SIZE: usize = 64 * 1024;

/// A task (thread) in the system
pub struct Task {
    /// Unique task ID
    id: TaskId,

    /// Current execution state
    state: TaskState,

    /// Saved CPU context
    context: TaskContext,

    /// Task's stack
    stack: Box<[u8; TASK_STACK_SIZE]>,

    /// Capability Space (security context)
    cspace: CSpace,

    /// Task priority
    priority: Priority,

    /// Task name (for debugging)
    name: &'static str,
}

impl Task {
    /// Create a new task with given entry point
    pub fn new(name: &'static str, entry_point: fn() -> !, priority: Priority) -> Self {
        use crate::scheduler::task_entry_wrapper;

        let mut context = TaskContext::new();

        // Allocate stack
        let stack = Box::new([0u8; TASK_STACK_SIZE]);

        // Set up initial context
        // RIP points to wrapper, which expects entry point in RDI
        context.rip = task_entry_wrapper as *const () as u64;
        context.rdi = entry_point as *const () as u64;  // Entry point in RDI for wrapper
        context.rsp = stack.as_ptr() as u64 + TASK_STACK_SIZE as u64;
        context.rbp = context.rsp;
        context.rflags = 0x200; // Enable interrupts (IF flag)

        static NEXT_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
        let id = TaskId::new(NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed));

        Task {
            id,
            state: TaskState::Ready,
            context,
            stack,
            cspace: CSpace::new(),
            priority,
            name,
        }
    }

    /// Get task ID
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get task state
    pub fn state(&self) -> TaskState {
        self.state
    }

    /// Set task state
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    /// Get mutable reference to context
    pub fn context_mut(&mut self) -> &mut TaskContext {
        &mut self.context
    }

    /// Get reference to context
    pub fn context(&self) -> &TaskContext {
        &self.context
    }

    /// Get task priority
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// Get task name
    pub fn name(&self) -> &str {
        self.name
    }

    /// Get mutable capability space
    pub fn cspace_mut(&mut self) -> &mut CSpace {
        &mut self.cspace
    }

    /// Get capability space
    pub fn cspace(&self) -> &CSpace {
        &self.cspace
    }
}

/// Task list for scheduler
pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    /// Create empty task list
    pub fn new() -> Self {
        TaskList {
            tasks: Vec::new(),
        }
    }

    /// Add a task to the list
    pub fn add(&mut self, task: Task) -> TaskId {
        let id = task.id();
        self.tasks.push(task);
        id
    }

    /// Get task by ID
    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get mutable task by ID
    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Remove task by ID
    pub fn remove(&mut self, id: TaskId) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    /// Get all tasks
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter()
    }

    /// Get all mutable tasks
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Task> {
        self.tasks.iter_mut()
    }

    /// Count of tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for TaskList {
    fn default() -> Self {
        Self::new()
    }
}
