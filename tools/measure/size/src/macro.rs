#![no_std]
#![no_main]

use core::hint::black_box;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
        black_box(black_box(workload::macro_idle as extern "C" fn(u32) -> u32)(16));
    black_box(black_box(workload::macro_feed_bottom as extern "C" fn(u32) -> u32)(16));
    black_box(black_box(workload::macro_feed_unrelated as extern "C" fn(u32) -> u32)(16));
    black_box(black_box(workload::macro_feed_top as extern "C" fn(u32) -> u32)(16));
    black_box(black_box(workload::macro_feed_flat as extern "C" fn(u32) -> u32)(16));
    black_box(black_box(workload::macro_route as extern "C" fn(u32) -> u32)(16));
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
