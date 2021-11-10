#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout
mod mono;

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [UARTE1])]
mod app {
    use super::mono::{ExtU32, MonoTimer};
    use nrf52840_hal::{
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        pac::TIMER0,
        prelude::*,
    };

    #[monotonic(binds = TIMER0, default = true)]
    type MyMono = MonoTimer<TIMER0>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = MonoTimer::new(ctx.device.TIMER0);
        let p0 = Parts::new(ctx.device.P0);
        let led = p0.p0_13.into_push_pull_output(Level::High).degrade();
        defmt::info!("Hello world!");
        blink::spawn().ok();
        (Shared {}, Local { led }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(local = [led])]
    fn blink(ctx: blink::Context) {
        defmt::info!("Blink!");
        let led = ctx.local.led;
        if led.is_set_low().unwrap() {
            led.set_high().ok();
        } else {
            led.set_low().ok();
        }
        blink::spawn_after(1.secs()).ok();
    }
}
