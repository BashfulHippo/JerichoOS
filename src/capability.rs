// capability system (seL4-style)
//
// basically tokens that prove you can access something
// cant be forged, cant be escalated - need to delegate properly

use alloc::collections::BTreeMap;
use spin::{Mutex, Once};

/// Unique capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityId(u64);

impl CapabilityId {
    /// Create a new capability ID
    pub fn new(id: u64) -> Self {
        CapabilityId(id)
    }

    /// Get the raw ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Capability rights/permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]  // ARM64: C layout (4 bools = 4 bytes)
pub struct Rights {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub grant: bool,  // Can grant this capability to others
}

impl Rights {
    /// No permissions
    pub const NONE: Rights = Rights {
        read: false,
        write: false,
        execute: false,
        grant: false,
    };

    /// Read-only
    pub const READ: Rights = Rights {
        read: true,
        write: false,
        execute: false,
        grant: false,
    };

    /// Read-write
    pub const READ_WRITE: Rights = Rights {
        read: true,
        write: true,
        execute: false,
        grant: false,
    };

    /// Full permissions
    pub const ALL: Rights = Rights {
        read: true,
        write: true,
        execute: true,
        grant: true,
    };

    /// Check if this capability has a specific right
    pub fn has(&self, other: Rights) -> bool {
        // tried bitflags for this originally but had issues
        (!other.read || self.read)
            && (!other.write || self.write)
            && (!other.execute || self.execute)
            && (!other.grant || self.grant)
    }

    /// Derive new rights (can only reduce, never increase)
    pub fn derive(&self, requested: Rights) -> Option<Rights> {
        if self.has(requested) {
            Some(requested)
        } else {
            None
        }
    }
}

/// Types of resources that can have capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]  // ARM64: Explicit 4-byte discriminant
pub enum ResourceType {
    Memory,
    Interrupt,
    Thread,
    Endpoint,  // For IPC
    WasmModule,
}

/// A capability token - unforgeable reference to a resource
#[derive(Debug, Clone)]
#[repr(C)]  // ARM64: C layout (removed align(16) - conflicts with BTreeMap)
pub struct Capability {
    id: CapabilityId,
    resource_type: ResourceType,
    resource_id: u64,  // Physical address, IRQ number, thread ID, etc.
    rights: Rights,
}

impl Capability {
    /// Create a new capability (only callable by kernel)
    pub fn new(id: CapabilityId, resource_type: ResourceType, resource_id: u64, rights: Rights) -> Self {
        Capability {
            id,
            resource_type,
            resource_id,
            rights,
        }
    }

    /// Get capability ID
    pub fn id(&self) -> CapabilityId {
        self.id
    }

    /// Get resource type
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Get resource ID
    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }

    /// Get rights
    pub fn rights(&self) -> Rights {
        self.rights
    }

    /// Derive a new capability with reduced rights
    pub fn derive(&self, new_id: CapabilityId, new_rights: Rights) -> Option<Capability> {
        self.rights.derive(new_rights).map(|rights| {
            Capability {
                id: new_id,
                resource_type: self.resource_type,
                resource_id: self.resource_id,
                rights,
            }
        })
    }
}

/// Capability Space (CSpace) - stores all capabilities for an entity
#[repr(C)]  // ARM64: Ensure consistent layout
pub struct CSpace {
    capabilities: BTreeMap<CapabilityId, Capability>,  // Restored BTreeMap
    next_id: u64,
}

impl CSpace {
    /// Create a new empty capability space
    pub fn new() -> Self {
        CSpace {
            capabilities: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Insert a capability into this CSpace
    pub fn insert(&mut self, capability: Capability) -> CapabilityId {
        let id = capability.id();
        self.capabilities.insert(id, capability);
        id
    }

    /// Get a capability by ID
    pub fn get(&self, id: CapabilityId) -> Option<&Capability> {
        self.capabilities.get(&id)
    }

    /// Remove a capability (revoke)
    pub fn revoke(&mut self, id: CapabilityId) -> Option<Capability> {
        self.capabilities.remove(&id)
    }

    /// Create a new capability in this CSpace
    pub fn create(&mut self, resource_type: ResourceType, resource_id: u64, rights: Rights) -> CapabilityId {
        let id = CapabilityId::new(self.next_id);
        self.next_id += 1;

        let cap = Capability::new(id, resource_type, resource_id, rights);
        self.insert(cap);
        id
    }

    /// Derive a new capability from an existing one (with reduced rights)
    pub fn derive(&mut self, source_id: CapabilityId, new_rights: Rights) -> Option<CapabilityId> {
        // TODO: should we audit derivations? could be useful for security analysis
        let source_cap = self.get(source_id)?.clone();

        let new_id = CapabilityId::new(self.next_id);
        self.next_id += 1;

        let derived_cap = source_cap.derive(new_id, new_rights)?;
        self.insert(derived_cap);
        Some(new_id)
    }

    /// Get number of capabilities
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// Global kernel capability space
/// Using spin::Once for modern, ARM64-safe lazy initialization
static KERNEL_CSPACE: Once<Mutex<CSpace>> = Once::new();

/// Initialize the kernel's capability space
/// Must be called after heap initialization
pub fn init() {
    KERNEL_CSPACE.call_once(|| {
        let cspace = CSpace::new();
        Mutex::new(cspace)
    });
}

/// Get a reference to the kernel CSpace
/// Panics if not initialized
pub fn kernel_cspace() -> &'static Mutex<CSpace> {
    KERNEL_CSPACE.get().expect("Capability system not initialized - call capability::init() first")
}

/// Create a new user CSpace with limited capabilities
pub fn create_user_cspace() -> CSpace {
    CSpace::new()
}
