#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use core::convert::TryInto;
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Input, Pin, PullUp},
        gpiote::Gpiote,
        prelude::*,
    };
    use rtic::time::{duration::Milliseconds, Instant};

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[resources]
    struct Resources {
        btn: Pin<Input<PullUp>>,
        gpiote: Gpiote,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();
        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let btn = p0.p0_11.into_pullup_input().degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote.port().input_pin(&btn).low();
        gpiote.port().enable_interrupt();

        defmt::info!("Press button!");
        (init::LateResources { btn, gpiote }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds = GPIOTE, resources = [gpiote])]
    fn on_gpiote(mut ctx: on_gpiote::Context) {
        ctx.resources.gpiote.lock(|gpiote| gpiote.reset_events());
        debounce::spawn_after(Milliseconds(30_u32)).ok();
    }
    #[task(resources = [btn])]

    fn debounce(mut ctx: debounce::Context) {
        static mut PRESSED_AT: Option<Instant<MyMono>> = None;
        if ctx.resources.btn.lock(|btn| btn.is_low().unwrap()) {
            PRESSED_AT.replace(monotonics::MyMono::now());
        } else {
            if let Some(instant) = PRESSED_AT.take() {
                let diff: Option<Milliseconds> = monotonics::MyMono::now()
                    .checked_duration_since(&instant)
                    .and_then(|d| d.try_into().ok());
                if let Some(Milliseconds(t)) = diff {
                    defmt::info!("Pressed for {} ms", t);
                }
            }
        }
    }
}
