# JerichoOS Architectural Decisions

This file documents significant architectural and security decisions.

---

## [2026-01-13] Interrupt Disable for Scheduler Lock Protection

**Context**: Timer interrupt could fire while `SCHEDULER` lock was held in `task_yield()`, causing deadlock. The timer handler also needs the scheduler lock.

**Decision**: Disable interrupts before acquiring `SCHEDULER` lock in `task_yield()`, re-enable after context switch completes.

**Alternatives Considered**:
- Try-lock in timer handler: Rejected because it would cause missed scheduling decisions
- Separate lock for timer: Rejected because timer needs to call `schedule()` which requires the same state
- Lock-free scheduler: Rejected as over-engineering for single-core design

**Consequences**:
- Positive: Deadlock eliminated, simple to reason about
- Negative: Slightly higher interrupt latency during context switch
- Constraints: This pattern must be used for any code that holds SCHEDULER lock

---

## [2026-01-13] Deny Capability Creation from Userspace

**Context**: `sys_cap_create()` syscall allowed any task to create capabilities with arbitrary rights to any resource, enabling complete security bypass.

**Decision**: Unconditionally deny `sys_cap_create()` from userspace. Return `PermissionDenied` for all calls.

**Alternatives Considered**:
- Privileged task check: Rejected because no privilege model exists yet
- Kernel-only flag: Rejected as it adds complexity without benefit
- Resource ownership check: Rejected because it requires additional infrastructure

**Consequences**:
- Positive: Capability forgery from userspace is impossible
- Negative: Tasks cannot create capabilities (must receive via delegation)
- Constraints: Kernel must provide initial capabilities at task creation

---

## [2026-01-13] Four-Layer Capability Verification for IPC

**Context**: IPC `send_message()` and `try_receive_message()` had no capability enforcement. Any task could send/receive on any endpoint.

**Decision**: Implement four-layer verification:
1. Verify caller holds the capability (exists in their CSpace)
2. Verify capability is for Endpoint resource type
3. Verify capability has required rights (WRITE for send, READ for receive)
4. Verify capability's resource_id matches target endpoint

**Alternatives Considered**:
- Simple ownership check: Rejected because it doesn't verify rights
- ACL-based: Rejected because it contradicts capability model
- Two-layer (possession + rights): Rejected because it allows type confusion

**Consequences**:
- Positive: Complete capability enforcement on IPC
- Negative: Function signatures changed, callers must provide CSpace
- Constraints: All IPC callers must have access to caller's CSpace

---

## [2026-01-13] Kernel-Assigned Sender Identity

**Context**: IPC messages need sender identification, but caller-provided sender ID could be spoofed.

**Decision**: Sender `TaskId` is a kernel-provided parameter, not caller-controlled. The kernel passes the true caller identity.

**Alternatives Considered**:
- Caller-provided with verification: Rejected because caller shouldn't know other task IDs
- Signed sender ID: Over-engineering for single-address-space kernel
- No sender ID: Rejected because receivers need to know message origin

**Consequences**:
- Positive: Sender spoofing is impossible
- Negative: Kernel must track and pass caller identity
- Constraints: Syscall layer must extract caller ID from task context, not arguments

---

## [2026-01-13] Full Capability Objects in WASM Context

**Context**: `host_sys_ipc_send` only checked `capabilities.is_empty()`, allowing any module with any capability to send IPC to any destination. A module with a Memory capability could message any endpoint.

**Decision**: Change `WasmContext.capabilities` from `Vec<CapabilityId>` to `Vec<Capability>`. Implement full 4-layer verification:
1. `find_capability(ResourceType, resource_id)` to locate matching capability
2. Verify `ResourceType::Endpoint` (implicit in find)
3. Verify `rights().write` for sending
4. Verify `resource_id` matches destination (implicit in find)

**Alternatives Considered**:
- Keep IDs, lookup in kernel CSpace: Rejected due to lock ordering complexity
- ACL per endpoint: Rejected as it contradicts capability model
- Trust any capability: Rejected as it defeats capability-based security

**Consequences**:
- Positive: WASM modules now require proper Endpoint capability with WRITE rights
- Negative: `grant_capability()` API changed to accept `Capability` instead of `CapabilityId`
- Constraints: Callers of `grant_capability` must construct full Capability objects

---

## [2026-01-13] Guest-Provided Buffers for IPC Delivery

**Context**: `deliver_pending_messages` wrote to fixed address 1024 in guest memory without consent. This could corrupt guest's stack, heap, or data.

**Decision**: Guest must export `allocate_message_buffer(size: i32) -> i32` function. Kernel calls this to get guest-allocated buffer pointer before writing. If function missing or returns invalid pointer, message delivery is skipped (safe default).

**Alternatives Considered**:
- Keep fixed address with documentation: Rejected as it violates memory safety principle
- Ring buffer protocol: Rejected as over-engineering for demo
- Kernel allocates in guest memory: Rejected because kernel shouldn't control guest layout

**Consequences**:
- Positive: Guest controls its own memory layout
- Positive: No kernel writes to fixed addresses in guest memory
- Negative: Guest must implement `allocate_message_buffer` to receive messages
- Constraints: Legacy guests without this function cannot receive IPC messages

---

## [2026-01-14] IPC Resource Limits for DoS Prevention

**Context**: WASM host functions `host_sys_ipc_send` and `host_sys_mqtt_publish` allowed unbounded message sizes and unlimited queue growth. A malicious module could exhaust kernel memory.

**Decision**: Implement two hard limits:
1. `MAX_IPC_MESSAGE_SIZE = 512 bytes` - Maximum size of any single IPC message
2. `MAX_IPC_QUEUE_DEPTH = 64 messages` - Maximum messages in global IPC queue

Checks are performed atomically: queue depth is verified under lock BEFORE allocating message buffer.

**Alternatives Considered**:
- Per-task quotas: Rejected as over-engineering for current single-queue design
- Dynamic limits based on memory pressure: Rejected due to complexity and unpredictable behavior
- Larger limits (4KB messages, 256 queue): Rejected to keep worst-case memory bounded at 32KB

**Consequences**:
- Positive: Kernel memory bounded at 32KB maximum for IPC queue
- Positive: DoS attacks via message flooding are prevented
- Negative: Legitimate large messages must be fragmented by application
- Negative: High-throughput scenarios may see EAGAIN errors requiring retry
- Constraints: Limits are compile-time constants; changing requires recompilation
