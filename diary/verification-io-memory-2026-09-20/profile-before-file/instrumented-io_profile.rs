
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
pub static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static REALLOCS: AtomicUsize = AtomicUsize::new(0);
pub static REALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static LIVE: AtomicUsize = AtomicUsize::new(0);
pub static PEAK: AtomicUsize = AtomicUsize::new(0);
pub static READS: AtomicUsize = AtomicUsize::new(0);
pub static READ_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static SCANNED: AtomicUsize = AtomicUsize::new(0);
pub static COPIED: AtomicUsize = AtomicUsize::new(0);
pub static COMPACTED: AtomicUsize = AtomicUsize::new(0);
pub static CAPACITY: AtomicUsize = AtomicUsize::new(0);
fn grow(n: usize) { let live = LIVE.fetch_add(n, Relaxed) + n; PEAK.fetch_max(live, Relaxed); }
struct Count;
#[global_allocator] static ALLOCATOR: Count = Count;
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() { ALLOCS.fetch_add(1, Relaxed); ALLOC_BYTES.fetch_add(l.size(), Relaxed); grow(l.size()); }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() { ALLOCS.fetch_add(1, Relaxed); ALLOC_BYTES.fetch_add(l.size(), Relaxed); grow(l.size()); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed); unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            REALLOCS.fetch_add(1, Relaxed); REALLOC_BYTES.fetch_add(n, Relaxed);
            if n >= l.size() { grow(n-l.size()); } else { LIVE.fetch_sub(l.size()-n, Relaxed); }
        }
        q
    }
}
pub fn position(s: &[u8], b: u8) -> Option<usize> {
    let p = s.iter().position(|v| *v == b);
    SCANNED.fetch_add(p.map_or(s.len(), |p| p+1), Relaxed); p
}
pub fn copy(s: &[u8]) -> Vec<u8> { COPIED.fetch_add(s.len(), Relaxed); s.to_vec() }
pub fn report() {
    for (name, value) in [("allocations", &ALLOCS), ("allocated_bytes", &ALLOC_BYTES),
        ("reallocations", &REALLOCS), ("reallocation_requested_bytes", &REALLOC_BYTES),
        ("peak_live_requested_bytes", &PEAK), ("physical_reads", &READS),
        ("read_bytes", &READ_BYTES), ("separator_bytes_examined", &SCANNED),
        ("reader_explicit_copy_bytes", &COPIED), ("compacted_bytes", &COMPACTED),
        ("max_reader_capacity", &CAPACITY)] {
        eprintln!("IOPROFILE {} {}", name, value.load(Relaxed));
    }
}
