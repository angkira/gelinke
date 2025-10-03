#![no_std]
#![no_main]

extern crate alloc;

mod firmware;

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    // Initialize heap for iRPC serialization
    {
        use core::mem::MaybeUninit;
        use core::ptr::addr_of_mut;
        const HEAP_SIZE: usize = 4096; // 4KB heap for iRPC messages
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }
    
    firmware::startup::run(spawner).await
}
