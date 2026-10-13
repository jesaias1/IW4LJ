//! Development heap probe (`--features heap-probe`).
//!
//! A phone gives no heap profiler to a release build, and "the game uses 4 GB"
//! says nothing about what to fix. This allocator wrapper records every live
//! allocation of [`THRESHOLD`] bytes or more with the call stack that made it,
//! and a background thread prints the largest holders every [`PERIOD`].
//!
//! The library is not extracted from the APK, so frames cannot be named on the
//! device; they are printed as offsets into the library and resolved on the
//! build machine (`llvm-addr2line -f -C -e libiw4l_android.so <offsets>`,
//! against an unstripped link: `CARGO_PROFILE_PLAY_STRIP=none`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{c_int, c_void};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

const THRESHOLD: usize = 64 * 1024;
const FRAMES: usize = 14;
const PERIOD: Duration = Duration::from_secs(75);

type Stack = [usize; FRAMES];

static ENABLED: AtomicBool = AtomicBool::new(false);
static LIVE: Mutex<Option<HashMap<usize, (usize, Stack)>>> = Mutex::new(None);
static SMALL_BYTES: AtomicUsize = AtomicUsize::new(0);
static SMALL_SEEN: AtomicUsize = AtomicUsize::new(0);
const SAMPLE: usize = 64;

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "C" {
    fn _Unwind_Backtrace(
        trace: extern "C" fn(*mut c_void, *mut c_void) -> c_int,
        arg: *mut c_void,
    ) -> c_int;
    fn _Unwind_GetIP(context: *mut c_void) -> usize;
    fn dladdr(address: *const c_void, info: *mut DlInfo) -> c_int;
}

#[repr(C)]
struct DlInfo {
    file_name: *const u8,
    base: *mut c_void,
    symbol_name: *const u8,
    symbol_address: *mut c_void,
}

struct Walk {
    stack: Stack,
    at: usize,
}

extern "C" fn step(context: *mut c_void, arg: *mut c_void) -> c_int {
    // SAFETY: `arg` is the `Walk` handed to `_Unwind_Backtrace` below.
    let walk = unsafe { &mut *arg.cast::<Walk>() };
    if walk.at >= FRAMES {
        return 5; // _URC_END_OF_STACK
    }
    // SAFETY: the unwinder passes a live context to its callback.
    walk.stack[walk.at] = unsafe { _Unwind_GetIP(context) };
    walk.at += 1;
    0
}

fn guarded(work: impl FnOnce()) {
    let _ = BUSY.try_with(|busy| {
        if busy.replace(true) {
            return;
        }
        work();
        busy.set(false);
    });
}

fn note(pointer: *mut u8, size: usize) {
    if pointer.is_null() || !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if size < THRESHOLD {
        SMALL_BYTES.fetch_add(size, Ordering::Relaxed);
        // One small allocation in `SAMPLE` is recorded with its stack and
        // stands for the rest when the report scales it back up.
        if SMALL_SEEN.fetch_add(1, Ordering::Relaxed) % SAMPLE != 0 {
            return;
        }
    }
    guarded(|| {
        let mut walk = Walk {
            stack: [0; FRAMES],
            at: 0,
        };
        // SAFETY: `step` only writes into `walk`.
        unsafe { _Unwind_Backtrace(step, (&raw mut walk).cast()) };
        if let Ok(mut live) = LIVE.lock() {
            live.get_or_insert_with(HashMap::new)
                .insert(pointer as usize, (size, walk.stack));
        }
    });
}

fn forget(pointer: *mut u8, size: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if size < THRESHOLD {
        // Saturating: the probe may have been switched on after the allocation.
        let _ = SMALL_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bytes| {
            Some(bytes.saturating_sub(size))
        });
    }
    guarded(|| {
        if let Ok(mut live) = LIVE.lock()
            && let Some(live) = live.as_mut()
        {
            live.remove(&(pointer as usize));
        }
    });
}

pub struct Probe;

// SAFETY: every call is forwarded to `System`; the bookkeeping never touches
// the returned memory.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        note(pointer, layout.size());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        note(pointer, layout.size());
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        forget(pointer, layout.size());
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        forget(pointer, layout.size());
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if moved.is_null() {
            note(pointer, layout.size());
        } else {
            note(moved, new_size);
        }
        moved
    }
}

/// Start recording and reporting. Called once at the top of `main`.
pub fn start() {
    ENABLED.store(true, Ordering::Relaxed);
    let _ = std::thread::Builder::new()
        .name("heap-probe".into())
        .spawn(|| {
            loop {
                std::thread::sleep(PERIOD);
                report();
            }
        });
}

fn report() {
    let mut info = DlInfo {
        file_name: std::ptr::null(),
        base: std::ptr::null_mut(),
        symbol_name: std::ptr::null(),
        symbol_address: std::ptr::null_mut(),
    };
    // SAFETY: `report` is an address inside this library; `info` is writable.
    unsafe { dladdr((report as fn()) as *const c_void, &raw mut info) };
    let base = info.base as usize;

    let mut sites: HashMap<Stack, (usize, usize)> = HashMap::new();
    let mut total = 0usize;
    guarded(|| {
        if let Ok(live) = LIVE.lock()
            && let Some(live) = live.as_ref()
        {
            for (size, stack) in live.values() {
                let small = *size < THRESHOLD;
                let size = if small { size * SAMPLE } else { *size };
                // Small allocations are grouped by their nearest callers only:
                // the full stack would scatter one cause over many rows.
                let mut stack = *stack;
                if small {
                    stack[8..].fill(0);
                }
                let site = sites.entry(stack).or_default();
                site.0 += size;
                site.1 += 1;
                total += size;
            }
        }
    });
    let mut sites = sites.into_iter().collect::<Vec<_>>();
    sites.sort_by_key(|(_, (bytes, _))| std::cmp::Reverse(*bytes));
    println!(
        "heap-probe: large={}MiB in {} sites, small={}MiB, base={base:#x}",
        total >> 20,
        sites.len(),
        SMALL_BYTES.load(Ordering::Relaxed) >> 20
    );
    for (stack, (bytes, count)) in sites.iter().filter(|(_, (bytes, _))| *bytes >= 128 << 10) {
        let frames = stack
            .iter()
            .skip(2)
            .filter(|ip| **ip != 0)
            .map(|ip| format!("{:x}", ip.wrapping_sub(base)))
            .collect::<Vec<_>>()
            .join(" ");
        println!("heap-probe: {:>7}KiB n={count:<5} {frames}", bytes >> 10);
    }
}
