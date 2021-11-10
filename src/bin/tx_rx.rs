#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [UARTE1])]
mod app {
    use dwt_systick_monotonic::{
        fugit::{MicrosDurationU32, TimerInstantU32},
        DwtSystick, ExtU32,
    };
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        gpiote::Gpiote,
        prelude::*,
    };
    const FREQ: u32 = 64_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<FREQ>;

    #[shared]
    struct Shared {
        tx_instant: Option<TimerInstantU32<FREQ>>,
    }

    #[local]
    struct Local {
        gpiote: Gpiote,
        tx_pin: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, FREQ);

        let p0 = Parts::new(ctx.device.P0);
        let rx_pin = p0.p0_11.into_pulldown_input().degrade();
        let tx_pin = p0.p0_13.into_push_pull_output(Level::Low).degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&rx_pin)
            .lo_to_hi()
            .enable_interrupt();

        tx::spawn().ok();

        (
            Shared { tx_instant: None },
            Local { gpiote, tx_pin },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(shared = [tx_instant], local = [tx_pin])]
    fn tx(mut ctx: tx::Context) {
        ctx.local.tx_pin.set_high().ok();
        cortex_m::asm::delay(640);
        ctx.local.tx_pin.set_low().ok();
        ctx.shared.tx_instant.lock(|t| t.replace(monotonics::now()));
        tx::spawn_after(100.millis()).ok();
    }

    #[task(binds = GPIOTE, shared = [tx_instant], local = [gpiote])]
    fn rx(mut ctx: rx::Context) {
        ctx.local.gpiote.reset_events();
        if let Some(instant) = ctx.shared.tx_instant.lock(|t| t.take()) {
            let t: MicrosDurationU32 = (monotonics::now() - instant).convert();
            defmt::info!("Distance: {} cm", t.ticks() as f32 / 58.0);
        }
    }
}
