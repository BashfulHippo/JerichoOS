// wasm runtime (wasmi interpreter)
// runs wasm modules in sandboxed environment with capability checks

use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use wasmi::*;
use crate::capability::{Capability, ResourceType};
use ::core::str::from_utf8;
use spin::Mutex;

/// Global message queue for MQTT demo IPC
/// Stores pending IPC messages to be delivered to subscribers
static IPC_MESSAGE_QUEUE: Mutex<VecDeque<IpcMessage>> = Mutex::new(VecDeque::new());

// resource limits to prevent dos attacks
pub const MAX_IPC_MESSAGE_SIZE: usize = 512;  // max message size
pub const MAX_IPC_QUEUE_DEPTH: usize = 64;    // max queue depth

/// IPC message for delivery
#[derive(Clone)]
pub struct IpcMessage {
    pub dest_client_id: u32,
    pub message: Vec<u8>,
}

/// Global subscriber registry for MQTT demo
/// Tracks which client IDs are subscribers
static MQTT_SUBSCRIBERS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Wasm module handle with cached instance for reuse
pub struct WasmModule {
    _module: Module,
    store: Store<WasmContext>,
    instance: Instance,
}

/// Wasm execution context with capability access
pub struct WasmContext {
    /// Capabilities available to this Wasm module (full objects for verification)
    pub capabilities: Vec<Capability>,
}

impl WasmContext {
    /// Create a new Wasm context with given capabilities
    pub fn new(capabilities: Vec<Capability>) -> Self {
        WasmContext { capabilities }
    }

    /// Find a capability by resource type and resource ID
    ///
    /// Returns the first matching capability, if any.
    pub fn find_capability(&self, resource_type: ResourceType, resource_id: u64) -> Option<&Capability> {
        self.capabilities.iter().find(|cap| {
            cap.resource_type() == resource_type && cap.resource_id() == resource_id
        })
    }

    /// Check if this context has any capabilities
    pub fn has_capabilities(&self) -> bool {
        !self.capabilities.is_empty()
    }
}

// simple print for testing
fn host_print(_caller: Caller<'_, WasmContext>, value: i32) {
    serial_println!("[WASM] Print called: {}", value);
}

// print string from wasm memory
fn host_sys_print(caller: Caller<'_, WasmContext>, msg_ptr: i32, msg_len: i32) {
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => {
            serial_println!("[WASM] sys_print: no memory export");
            return;
        }
    };

    let msg_ptr = msg_ptr as usize;
    let msg_len = msg_len as usize;

    // Read bytes from WASM memory
    let data = memory.data(&caller);
    if msg_ptr + msg_len > data.len() {
        serial_println!("[WASM] sys_print: invalid memory access");
        return;
    }

    let msg_bytes = &data[msg_ptr..msg_ptr + msg_len];

    // Convert to string (lossy for non-UTF8)
    if let Ok(s) = from_utf8(msg_bytes) {
        serial_print!("{}", s);
    } else {
        serial_print!("[WASM] <invalid UTF-8>");
    }
}

// print u32 - arm64 uart doesn't support format args yet, so just print placeholder
fn host_sys_print_u32(_caller: Caller<'_, WasmContext>, _value: u32) {
    serial_print!("<u32>");
}

/// Host function: MQTT subscribe
fn host_sys_mqtt_subscribe(
    caller: Caller<'_, WasmContext>,
    client_id: u32,
    topic_ptr: i32,
    topic_len: i32,
) -> i32 {
    // Read topic from WASM memory
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return -1,
    };

    let data = memory.data(&caller);
    let topic_ptr = topic_ptr as usize;
    let topic_len = topic_len as usize;

    if topic_ptr + topic_len > data.len() {
        return -1;
    }

    let topic = &data[topic_ptr..topic_ptr + topic_len];

    serial_print!("[MQTT-SYSCALL] Subscribe: client_id=");
    serial_print!("<u32>");
    serial_print!(" topic=");
    if let Ok(s) = from_utf8(topic) {
        serial_print!("{}", s);
    }
    serial_print!("\n");

    // Register subscriber in global registry
    let mut subscribers = MQTT_SUBSCRIBERS.lock();
    if !subscribers.contains(&client_id) {
        subscribers.push(client_id);
    }

    // TODO: route to actual broker module instead of global registry
    0
}

// mqtt publish - enforces 512 byte message limit and 64 message queue depth
fn host_sys_mqtt_publish(
    caller: Caller<'_, WasmContext>,
    topic_ptr: i32,
    topic_len: i32,
    msg_ptr: i32,
    msg_len: i32,
) -> i32 {
    // reject huge messages (512 byte limit)
    let msg_len_usize = msg_len as usize;
    if msg_len < 0 || msg_len_usize > MAX_IPC_MESSAGE_SIZE {
        serial_println!("[MQTT-DENIED] Message too large: {} > {}", msg_len, MAX_IPC_MESSAGE_SIZE);
        return -4; // too big
    }

    // read topic and message from wasm memory
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return -1,
    };

    let data = memory.data(&caller);
    let topic_ptr = topic_ptr as usize;
    let topic_len = topic_len as usize;
    let msg_ptr = msg_ptr as usize;

    // Overflow-safe bounds check
    if topic_ptr.saturating_add(topic_len) > data.len()
        || msg_ptr.saturating_add(msg_len_usize) > data.len() {
        return -3; // EFAULT
    }

    let topic = &data[topic_ptr..topic_ptr + topic_len];
    let msg = &data[msg_ptr..msg_ptr + msg_len_usize];

    #[cfg(debug_assertions)]
    {
        serial_print!("[MQTT-SYSCALL] Publish: topic=");
        if let Ok(s) = from_utf8(topic) {
            serial_print!("{}", s);
        }
        serial_print!(" msg=");
        if let Ok(s) = from_utf8(msg) {
            serial_print!("{}", s);
        }
        serial_print!("\n");
    }
    let _ = topic; // Used in debug builds

    // Simplified broker: directly enqueue to all registered subscribers
    let subscribers = MQTT_SUBSCRIBERS.lock();
    let subscriber_count = subscribers.len();

    for &client_id in subscribers.iter() {
        // don't let queue grow forever - cap at 64 msgs
        let mut queue = IPC_MESSAGE_QUEUE.lock();
        if queue.len() >= MAX_IPC_QUEUE_DEPTH {
            serial_println!("[MQTT-DENIED] Queue full ({}/{})", queue.len(), MAX_IPC_QUEUE_DEPTH);
            break; // Stop enqueueing, return partial count
        }

        let ipc_msg = IpcMessage {
            dest_client_id: client_id,
            message: msg.to_vec(),
        };
        queue.push_back(ipc_msg);
    }

    subscriber_count as i32
}

/// Host function: IPC send - enqueues message for delivery
/// Enforces capability-based access control with 4-layer verification
///
/// # Security (4-Layer Capability Check)
/// 1. Find capability for destination endpoint
/// 2. Verify ResourceType::Endpoint
/// 3. Verify WRITE rights
/// 4. Verify resource_id matches destination
///
/// # Security (DoS Prevention)
/// - Message size limited to MAX_IPC_MESSAGE_SIZE (512 bytes)
/// - Queue depth limited to MAX_IPC_QUEUE_DEPTH (64 messages)
/// - Queue check happens BEFORE allocation to prevent memory exhaustion
///
/// # Assumptions
/// - TRUST: Called from WASM sandbox (untrusted code)
/// - Destination is treated as endpoint resource_id
fn host_sys_ipc_send(
    caller: Caller<'_, WasmContext>,
    dest: u32,
    msg_ptr: i32,
    msg_len: i32,
) -> i32 {
    // reject huge messages early (512 byte limit)
    let msg_len_usize = msg_len as usize;
    if msg_len < 0 || msg_len_usize > MAX_IPC_MESSAGE_SIZE {
        serial_println!("[IPC-DENIED] Message too large: {} > {}", msg_len, MAX_IPC_MESSAGE_SIZE);
        return -4; // too big
    }

    // verify caller has the right capability for this endpoint
    let cap = match caller.data().find_capability(ResourceType::Endpoint, dest as u64) {
        Some(c) => c,
        None => {
            serial_println!("[IPC-DENIED] No Endpoint capability for destination {}", dest);
            return -1; // EACCES: Permission denied
        }
    };

    // Layer 3: Verify WRITE rights (required for sending)
    if !cap.rights().write {
        serial_println!("[IPC-DENIED] Capability lacks WRITE rights for endpoint {}", dest);
        return -2; // EPERM: Operation not permitted
    }

    // Layer 4: Verify resource_id matches destination (already done in find_capability)
    // This is implicit in the find_capability call above

    // === Memory Access (after capability check passes) ===
    let memory = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return -3, // EFAULT: Bad address
    };

    let data = memory.data(&caller);
    let msg_ptr = msg_ptr as usize;

    // Bounds check with overflow protection (msg_len_usize already validated above)
    if msg_ptr.saturating_add(msg_len_usize) > data.len() {
        serial_println!("[IPC-DENIED] Invalid memory access: ptr={}, len={}", msg_ptr, msg_len_usize);
        return -3; // EFAULT: Bad address
    }

    let msg = &data[msg_ptr..msg_ptr + msg_len_usize];

    #[cfg(debug_assertions)]
    {
        serial_print!("[IPC-SYSCALL] Send to endpoint {} msg=", dest);
        if let Ok(s) = from_utf8(msg) {
            serial_print!("{}", s);
        }
        serial_print!("\n");
    }

    // check queue isn't full before we allocate
    let mut queue = IPC_MESSAGE_QUEUE.lock();
    if queue.len() >= MAX_IPC_QUEUE_DEPTH {
        serial_println!("[IPC-DENIED] Queue full: {} >= {}", queue.len(), MAX_IPC_QUEUE_DEPTH);
        return -5; // queue full, try again later
    }

    // good to go
    let ipc_msg = IpcMessage {
        dest_client_id: dest,
        message: msg.to_vec(),
    };
    queue.push_back(ipc_msg);

    0 // Success
}

impl WasmModule {
    /// Load a Wasm module from bytes and create a reusable instance
    pub fn from_bytes(wasm_bytes: &[u8]) -> Result<Self, Error> {
        // Create engine
        let engine = Engine::default();

        // Parse and validate module
        let module = Module::new(&engine, wasm_bytes)?;

        // Create store with context
        let context = WasmContext::new(Vec::new());
        let mut store = Store::new(&engine, context);

        // Create linker with host functions
        let linker = Self::create_linker(&engine);

        // Instantiate module once and cache it for reuse
        let instance = linker
            .instantiate(&mut store, &module)?
            .start(&mut store)?;

        Ok(WasmModule {
            _module: module,
            store,
            instance,
        })
    }

    /// Create a linker with host functions
    fn create_linker(engine: &Engine) -> Linker<WasmContext> {
        let mut linker = Linker::new(engine);

        // Add host function: print (original for i32)
        linker
            .func_wrap("env", "print", host_print)
            .expect("Failed to link print function");

        // mqtt syscalls for demos
        linker
            .func_wrap("env", "sys_print", host_sys_print)
            .expect("Failed to link sys_print");

        linker
            .func_wrap("env", "sys_print_u32", host_sys_print_u32)
            .expect("Failed to link sys_print_u32");

        linker
            .func_wrap("env", "sys_mqtt_subscribe", host_sys_mqtt_subscribe)
            .expect("Failed to link sys_mqtt_subscribe");

        linker
            .func_wrap("env", "sys_mqtt_publish", host_sys_mqtt_publish)
            .expect("Failed to link sys_mqtt_publish");

        linker
            .func_wrap("env", "sys_ipc_send", host_sys_ipc_send)
            .expect("Failed to link sys_ipc_send");

        linker
    }

    /// Call a function on the cached instance (no re-instantiation!)
    pub fn call_function(&mut self, func_name: &str, args: &[Value]) -> Result<Option<Value>, &'static str> {
        // Get the function from the cached instance
        let func = self.instance
            .get_func(&mut self.store, func_name)
            .ok_or("Function not found")?;

        // Get function type to determine result count
        let func_type = func.ty(&self.store);
        let result_count = func_type.results().len();

        // Allocate results buffer based on actual return type
        let mut results = vec![Value::I32(0); result_count];
        func.call(&mut self.store, args, &mut results)
            .map_err(|_| "Failed to call function")?;

        Ok(results.into_iter().next())
    }

    /// Add a capability to this module's context
    ///
    /// Grants the full capability object (not just ID) to enable
    /// proper 4-layer verification in host functions.
    pub fn grant_capability(&mut self, capability: Capability) {
        serial_println!("[WASM] Granted {:?} capability for resource {}",
            capability.resource_type(), capability.resource_id());
        self.store.data_mut().capabilities.push(capability);
    }

    /// Get capabilities count
    pub fn capability_count(&self) -> usize {
        self.store.data().capabilities.len()
    }
}

/// Initialize the Wasm runtime
pub fn init() {
    serial_println!("[WASM] Runtime initialized (wasmi interpreter)");
}

/// Load and validate a WASM module from bytes
pub fn load_and_validate(wasm_bytes: &[u8]) -> Result<WasmModule, Error> {
    WasmModule::from_bytes(wasm_bytes)
}

/// Deliver pending IPC messages to a subscriber module
/// Returns number of messages delivered
///
/// # Security
/// - Kernel NEVER writes to guest memory at fixed addresses
/// - Guest must export `allocate_message_buffer(size) -> ptr` to provide buffer
/// - If guest doesn't export this function, messages are not delivered (safe default)
///
/// # Assumptions
/// - TRUST: Guest code is untrusted
/// - Guest controls its own memory layout
pub fn deliver_pending_messages(subscriber: &mut WasmModule, client_id: u32) -> usize {
    let mut delivered = 0;

    // Drain all messages for this client from the queue
    loop {
        let msg_opt = {
            let mut queue = IPC_MESSAGE_QUEUE.lock();
            // Find first message for this client
            if let Some(pos) = queue.iter().position(|m| m.dest_client_id == client_id) {
                queue.remove(pos)
            } else {
                None
            }
        };

        match msg_opt {
            Some(ipc_msg) => {
                let msg_len = ipc_msg.message.len().min(MAX_IPC_MESSAGE_SIZE);

                // === SECURITY: Request buffer from guest (never use fixed address) ===
                // Guest must export allocate_message_buffer(size) -> ptr
                let buffer_ptr = match subscriber.call_function(
                    "allocate_message_buffer",
                    &[Value::I32(msg_len as i32)]
                ) {
                    Ok(Some(Value::I32(ptr))) if ptr > 0 => ptr,
                    Ok(Some(Value::I32(ptr))) => {
                        // Guest returned null/invalid pointer - skip this message
                        serial_println!("[IPC] Guest returned invalid buffer ptr: {}", ptr);
                        continue;
                    }
                    Ok(_) => {
                        // Wrong return type
                        serial_println!("[IPC] allocate_message_buffer returned unexpected type");
                        continue;
                    }
                    Err(_) => {
                        // Function doesn't exist or failed - safe default is to skip
                        serial_println!("[IPC] Guest doesn't export allocate_message_buffer - skipping delivery");
                        // Re-queue the message so it's not lost
                        let mut queue = IPC_MESSAGE_QUEUE.lock();
                        queue.push_front(ipc_msg);
                        break; // Stop trying for this subscriber
                    }
                };

                // Get subscriber's memory
                let memory = match subscriber.instance.get_export(&mut subscriber.store, "memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => {
                        serial_println!("[IPC] Subscriber has no memory export");
                        break;
                    }
                };

                // Write message to guest-provided buffer
                {
                    let data = memory.data_mut(&mut subscriber.store);
                    let buffer_start = buffer_ptr as usize;

                    // Bounds check on guest-provided pointer
                    if buffer_start.saturating_add(msg_len) > data.len() {
                        serial_println!("[IPC] Guest buffer out of bounds: ptr={}, len={}, mem_size={}",
                            buffer_start, msg_len, data.len());
                        continue;
                    }

                    data[buffer_start..buffer_start + msg_len]
                        .copy_from_slice(&ipc_msg.message[..msg_len]);
                }

                // Call subscriber_receive(msg_ptr, msg_len)
                let result = subscriber.call_function(
                    "subscriber_receive",
                    &[Value::I32(buffer_ptr), Value::I32(msg_len as i32)]
                );

                match result {
                    Ok(_) => {
                        delivered += 1;
                    }
                    Err(e) => {
                        serial_print!("[IPC] Failed to deliver message: ");
                        serial_println!("{}", e);
                    }
                }
            }
            None => break, // No more messages for this client
        }
    }

    delivered
}

/// Get count of pending messages for a client
pub fn pending_message_count(client_id: u32) -> usize {
    let queue = IPC_MESSAGE_QUEUE.lock();
    queue.iter().filter(|m| m.dest_client_id == client_id).count()
}

/// Clear all pending messages (for cleanup)
pub fn clear_ipc_queue() {
    let mut queue = IPC_MESSAGE_QUEUE.lock();
    queue.clear();
}
