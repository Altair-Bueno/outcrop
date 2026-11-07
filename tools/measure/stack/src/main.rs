//! Peak stack usage by stack painting, and native time per round.

use core::hint::black_box;
use std::time::Instant;

const PATTERN: u8 = 0xA5;
const MARGIN: usize = 64;
const DEPTH: usize = 512 * 1024;

#[inline(never)]
fn stack_pointer() -> usize {
    let marker = 0u8;
    black_box(&marker) as *const u8 as usize
}

#[inline(never)]
fn paint(top: usize) {
    for addr in top - DEPTH..top - MARGIN {
        // SAFETY: unused memory below the current frame, within the thread's stack.
        unsafe { core::ptr::write_volatile(addr as *mut u8, PATTERN) };
    }
}

#[inline(never)]
fn deepest(top: usize) -> usize {
    for addr in top - DEPTH..top - MARGIN {
        // SAFETY: as in `paint`.
        if unsafe { core::ptr::read_volatile(addr as *const u8) } != PATTERN {
            return top - addr;
        }
    }
    0
}

fn stack(f: extern "C" fn(u32) -> u32) -> usize {
    std::thread::Builder::new()
        .stack_size(4 * DEPTH)
        .spawn(move || {
            let top = stack_pointer();
            paint(top);
            black_box(f)(black_box(16));
            deepest(top)
        })
        .unwrap()
        .join()
        .unwrap()
}

fn fastest(f: extern "C" fn(u32) -> u32, rounds: u32) -> f64 {
    (0..500)
        .map(|_| {
            let start = Instant::now();
            black_box(black_box(f)(black_box(rounds)));
            start.elapsed().as_nanos() as f64
        })
        .fold(f64::INFINITY, f64::min)
}

extern "C" fn noop(_: u32) -> u32 {
    0
}

fn main() {
    let baseline = stack(noop);
    let (builder, macro_) = workload::sizes();
    println!("formation size: builder {builder} B, macro {macro_} B");
    println!("  scenario\tpeak stack B\tns/round");
    for (name, f) in workload::SCENARIOS {
        let ns = (fastest(f, 64) - fastest(f, 32)) / 32.0;
        println!("  {name}\t{}\t{ns:.1}", stack(f).saturating_sub(baseline));
    }
}
