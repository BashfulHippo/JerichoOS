//! System call interface for JerichoOS
//!
//! Provides the interface between user code and kernel services
//! All operations on capabilities go through syscalls

use crate::capability::{CapabilityId, Rights, ResourceType, CSpace};

/// Syscall numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    /// Create a new capability
    CapCreate = 0,
    /// Derive a capability with reduced rights
    CapDerive = 1,
    /// Revoke a capability
    CapRevoke = 2,
    /// Invoke a capability (use the resource it points to)
    CapInvoke = 3,
    /// Print to serial (for testing)
    Print = 100,
}

impl SyscallNumber {
    /// Convert from u64
    pub fn from_u64(n: u64) -> Option<Self> {
        match n {
            0 => Some(SyscallNumber::CapCreate),
            1 => Some(SyscallNumber::CapDerive),
            2 => Some(SyscallNumber::CapRevoke),
            3 => Some(SyscallNumber::CapInvoke),
            100 => Some(SyscallNumber::Print),
            _ => None,
        }
    }
}

/// Syscall result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallResult {
    Success(u64),
    Error(SyscallError),
}

/// Syscall errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidCapability,
    PermissionDenied,
    InvalidArgument,
}

/// Syscall context - simulates user process state
pub struct SyscallContext {
    /// The CSpace of the calling "process"
    pub cspace: CSpace,
}

impl SyscallContext {
    /// Create a new syscall context with an empty CSpace
    pub fn new() -> Self {
        SyscallContext {
            cspace: CSpace::new(),
        }
    }

    /// Handle a syscall
    pub fn syscall(
        &mut self,
        syscall_num: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
    ) -> SyscallResult {
        let syscall = match SyscallNumber::from_u64(syscall_num) {
            Some(s) => s,
            None => return SyscallResult::Error(SyscallError::InvalidSyscall),
        };

        match syscall {
            SyscallNumber::CapCreate => self.sys_cap_create(arg1, arg2, arg3),
            SyscallNumber::CapDerive => self.sys_cap_derive(arg1, arg2),
            SyscallNumber::CapRevoke => self.sys_cap_revoke(arg1),
            SyscallNumber::CapInvoke => self.sys_cap_invoke(arg1, arg2, arg3, arg4),
            SyscallNumber::Print => self.sys_print(arg1),
        }
    }

    /// Create a new capability
    /// arg1: resource_type (as u64)
    /// arg2: resource_id
    /// arg3: rights (encoded as bitflags)
    fn sys_cap_create(&mut self, resource_type: u64, resource_id: u64, rights_bits: u64) -> SyscallResult {
        let resource_type = match resource_type {
            0 => ResourceType::Memory,
            1 => ResourceType::Interrupt,
            2 => ResourceType::Thread,
            3 => ResourceType::Endpoint,
            4 => ResourceType::WasmModule,
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };

        let rights = Rights {
            read: (rights_bits & 0x1) != 0,
            write: (rights_bits & 0x2) != 0,
            execute: (rights_bits & 0x4) != 0,
            grant: (rights_bits & 0x8) != 0,
        };

        let cap_id = self.cspace.create(resource_type, resource_id, rights);
        SyscallResult::Success(cap_id.value())
    }

    /// Derive a capability with reduced rights
    /// arg1: source capability ID
    /// arg2: new rights (encoded as bitflags)
    fn sys_cap_derive(&mut self, source_id: u64, rights_bits: u64) -> SyscallResult {
        let source_cap_id = CapabilityId::new(source_id);

        let new_rights = Rights {
            read: (rights_bits & 0x1) != 0,
            write: (rights_bits & 0x2) != 0,
            execute: (rights_bits & 0x4) != 0,
            grant: (rights_bits & 0x8) != 0,
        };

        match self.cspace.derive(source_cap_id, new_rights) {
            Some(new_id) => SyscallResult::Success(new_id.value()),
            None => SyscallResult::Error(SyscallError::PermissionDenied),
        }
    }

    /// Revoke a capability
    /// arg1: capability ID
    fn sys_cap_revoke(&mut self, cap_id: u64) -> SyscallResult {
        let cap_id = CapabilityId::new(cap_id);

        match self.cspace.revoke(cap_id) {
            Some(_) => SyscallResult::Success(0),
            None => SyscallResult::Error(SyscallError::InvalidCapability),
        }
    }

    /// Invoke a capability (use the resource)
    /// arg1: capability ID
    /// arg2-4: operation-specific arguments
    fn sys_cap_invoke(&mut self, cap_id: u64, _arg2: u64, _arg3: u64, _arg4: u64) -> SyscallResult {
        let cap_id = CapabilityId::new(cap_id);

        match self.cspace.get(cap_id) {
            Some(cap) => {
                // In a real implementation, this would perform the actual operation
                // For now, just verify the capability exists and has rights
                serial_println!("[SYSCALL] Invoked capability {} for {:?} resource {}",
                    cap.id().value(), cap.resource_type(), cap.resource_id());
                SyscallResult::Success(1)
            }
            None => SyscallResult::Error(SyscallError::InvalidCapability),
        }
    }

    /// Print syscall (for testing)
    /// arg1: value to print
    fn sys_print(&mut self, value: u64) -> SyscallResult {
        serial_println!("[SYSCALL] Print: {}", value);
        SyscallResult::Success(0)
    }

    /// Get the number of capabilities in this context's CSpace
    pub fn capability_count(&self) -> usize {
        self.cspace.len()
    }
}

impl Default for SyscallContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to encode rights as u64 for syscalls
pub fn encode_rights(rights: Rights) -> u64 {
    let mut bits = 0u64;
    if rights.read { bits |= 0x1; }
    if rights.write { bits |= 0x2; }
    if rights.execute { bits |= 0x4; }
    if rights.grant { bits |= 0x8; }
    bits
}
