//! Heap allocator for JerichoOS
//!
//! Provides dynamic memory allocation using a linked list allocator

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Heap start address
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// Heap size: 8 MB (both architectures)
///
/// Step 2A Investigation (2025-12-28):
/// - Root cause: linked_list_allocator fragmentation prevents large contiguous allocations
/// - Tested: 512KB, 1MB, 2MB all fail at Demo 4 (MQTT subscriber needs 1.06 MB)
/// - Solution: 8 MB heap provides sufficient headroom for fragmentation
/// - ARM64: Proven with all 5 demos passing
/// - x86-64: Option A (ARM64 parity) chosen over allocator replacement (Option B)
///
/// Known limitation: Simple linked-list allocator may fragment over time.
/// Future enhancement: Replace with buddy/slab/TLSF allocator (Phase 2).
pub const HEAP_SIZE: usize = 8 * 1024 * 1024;

/// Initialize the heap allocator
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // Map heap pages
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + (HEAP_SIZE as u64) - 1u64;
        let heap_start_page: Page<Size4KiB> = Page::containing_address(heap_start);
        let heap_end_page: Page<Size4KiB> = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    // Initialize the allocator
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}

/// Dummy allocator for #[alloc_error_handler]
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
