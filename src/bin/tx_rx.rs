#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use core::convert::TryInto;
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        gpiote::Gpiote,
        prelude::*,
    };
    use rtic::time::{duration::Milliseconds, Instant};

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[resources]
    struct Resources {
        gpiote: Gpiote,
        tx_pin: Pin<Output<PushPull>>,
        #[init(None)]
        tx_instant: Option<Instant<MyMono>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

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
            init::LateResources { gpiote, tx_pin },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(resources = [tx_pin, tx_instant])]
    fn tx(mut ctx: tx::Context) {
        ctx.resources.tx_pin.lock(|pin| {
            pin.set_high().ok();
            cortex_m::asm::delay(640);
            pin.set_low().ok();
        });
        ctx.resources
            .tx_instant
            .lock(|t| t.replace(monotonics::MyMono::now()));
        tx::spawn_after(Milliseconds(100_u32)).ok();
    }

    #[task(binds = GPIOTE, resources = [gpiote, tx_instant])]
    fn rx(mut ctx: rx::Context) {
        ctx.resources.gpiote.lock(|gpiote| gpiote.reset_events());
        if let Some(instant) = ctx.resources.tx_instant.lock(|t| t.take()) {
            let diff: Option<Milliseconds> = monotonics::MyMono::now()
                .checked_duration_since(&instant)
                .and_then(|dur| dur.try_into().ok());
            if let Some(Milliseconds(t)) = diff {
                defmt::info!("Echo received after {} ms", t);
            }
        }
    }
}
