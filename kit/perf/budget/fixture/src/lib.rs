//! kit/perf-budget-test.sh's fixture library: exports whose fast paths
//! return and whose slow paths tail-call `slow`, and a flag byte that
//! the test's table names as a configuration cell. `fx_set` writes the
//! flag, so the fast paths must read it.

use core::sync::atomic::{AtomicU8, Ordering};

static FLAGS: AtomicU8 = AtomicU8::new(0);

#[inline(never)]
#[cold]
fn slow(a: u64, b: u64) -> u64 {
    core::hint::black_box(a ^ b)
}

/// `b` when no flag is set (`a + b` with the `wide` feature), else the
/// slow path.
#[unsafe(no_mangle)]
pub extern "C" fn fx_pick(a: u64, b: u64) -> u64 {
    if FLAGS.load(Ordering::Relaxed) != 0 {
        return slow(a, b);
    }
    if cfg!(feature = "wide") {
        a.wrapping_add(b)
    } else {
        b
    }
}

/// The word `p` points at, plus one, when flag bit 0 is clear, else the
/// slow path.
#[unsafe(no_mangle)]
pub extern "C" fn fx_load(p: &u64, b: u64) -> u64 {
    if FLAGS.load(Ordering::Relaxed) & 1 != 0 {
        return slow(*p, b);
    }
    p.wrapping_add(1)
}

/// Sets the flags.
#[unsafe(no_mangle)]
pub extern "C" fn fx_set(v: u8) {
    FLAGS.store(v, Ordering::Relaxed);
}
