#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        extern "C" {
            static MSG: [u8; 16];
        }
        core::arch::asm!(
            "adrp x1, {msg}\n add x1, x1, :lo12:{msg}\n mov x2, #15\n mov x0, #1\n svc #1\n mov x0, #0\n svc #2",
            msg = sym MSG,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub static MSG: [u8; 16] = [
    b'H', b'e', b'l', b'l', b'o', b' ', b'f', b'r', b'o', b'm', b' ', b'E', b'L', b'F', b'\n', 0
];
