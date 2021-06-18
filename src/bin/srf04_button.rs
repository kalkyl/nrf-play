#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use core::convert::TryInto;
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Input, Level, Output, Pin, PullDown, PullUp, PushPull},
        gpiote::Gpiote,
        prelude::*,
    };
    use rtic::time::{
        duration::{Microseconds, Milliseconds},
        Instant,
    };

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[resources]
    struct Resources {
        btn: Pin<Input<PullUp>>,
        gpiote: Gpiote,
        rx_pin: Pin<Input<PullDown>>,
        tx_pin: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let rx_pin = p0.p0_04.into_pulldown_input().degrade();
        let tx_pin = p0.p0_03.into_push_pull_output(Level::Low).degrade();
        let btn = p0.p0_11.into_pullup_input().degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&rx_pin)
            .toggle()
            .enable_interrupt();
        gpiote
            .channel1()
            .input_pin(&btn)
            .hi_to_lo()
            .enable_interrupt();

        (
            init::LateResources {
                btn,
                gpiote,
                rx_pin,
                tx_pin,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds = GPIOTE, resources = [gpiote, rx_pin])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        static mut START: Option<Instant<MyMono>> = None;
        (ctx.resources.gpiote, ctx.resources.rx_pin).lock(|gpiote, rx_pin| {
            if gpiote.channel0().is_event_triggered() {
                gpiote.reset_events();
                if rx_pin.is_high().unwrap() {
                    START.replace(monotonics::MyMono::now());
                } else {
                    if let Some(instant) = START.take() {
                        let diff: Option<Microseconds> = monotonics::MyMono::now()
                            .checked_duration_since(&instant)
                            .and_then(|dur| dur.try_into().ok());
                        if let Some(Microseconds(t)) = diff {
                            defmt::info!("Pulse length: {} us", t);
                        }
                    }
                }
            } else {
                gpiote.reset_events();
                debounce::spawn_after(Milliseconds(30_u32)).ok();
            }
        });
    }

    #[task(resources = [btn, tx_pin])]
    fn debounce(mut ctx: debounce::Context) {
        if ctx.resources.btn.lock(|btn| btn.is_low().unwrap()) {
            ctx.resources.tx_pin.lock(|pin| {
                pin.set_high().ok();
                cortex_m::asm::delay(640);
                pin.set_low().ok();
            });
        }
    }
}
