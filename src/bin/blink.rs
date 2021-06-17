#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        prelude::*,
    };
    use rtic::time::duration::Seconds;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[resources]
    struct Resources {
        led: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let led = p0.p0_13.into_push_pull_output(Level::High).degrade();

        defmt::info!("Hello world!");
        blink::spawn_after(Seconds(1_u32)).ok();
        (init::LateResources { led }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(resources = [led])]
    fn blink(mut ctx: blink::Context) {
        defmt::info!("Blink!");
        ctx.resources.led.lock(|led| {
            if led.is_set_low().unwrap() {
                led.set_high().ok();
            } else {
                led.set_low().ok();
            }
        });
        blink::spawn_after(Seconds(1_u32)).ok();
    }
}
