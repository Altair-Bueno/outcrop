#![no_std]
#![no_main]

#[allow(unused_imports)]
use core::hint::black_box;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    #[allow(unused_extern_crates)]
    extern crate workload;
    
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
