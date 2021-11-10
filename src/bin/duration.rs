#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [UARTE1])]
mod app {
    use dwt_systick_monotonic::{
        fugit::{MillisDurationU32, TimerInstantU32},
        DwtSystick, ExtU32,
    };
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Input, Pin, PullUp},
        gpiote::Gpiote,
        prelude::*,
    };
    const FREQ: u32 = 64_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<FREQ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        btn: Pin<Input<PullUp>>,
        gpiote: Gpiote,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, FREQ);

        let p0 = Parts::new(ctx.device.P0);
        let btn = p0.p0_11.into_pullup_input().degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&btn)
            .toggle() // Trigger on both rising and falling edges
            .enable_interrupt();

        defmt::info!("Press button 1!");
        (Shared {}, Local { btn, gpiote }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(binds = GPIOTE, local = [gpiote])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        ctx.local.gpiote.reset_events();
        debounce::spawn_after(30.millis()).ok();
    }

    #[task(local = [btn, pressed_at: Option<TimerInstantU32<FREQ>> = None])]
    fn debounce(ctx: debounce::Context) {
        if ctx.local.btn.is_low().unwrap() {
            ctx.local.pressed_at.replace(monotonics::now());
        } else {
            if let Some(instant) = ctx.local.pressed_at.take() {
                let t: MillisDurationU32 = (monotonics::now() - instant).convert();
                defmt::info!("Pressed for {} ms", t.ticks());
            }
        }
    }
}
