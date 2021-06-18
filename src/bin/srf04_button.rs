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
        gpiote: Gpiote,
        btn: Pin<Input<PullUp>>,
        echo_pin: Pin<Input<PullDown>>,
        trig_pin: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let btn = p0.p0_11.into_pullup_input().degrade();
        let echo_pin = p0.p0_04.into_pulldown_input().degrade();
        let trig_pin = p0.p0_03.into_push_pull_output(Level::Low).degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&echo_pin)
            .toggle()
            .enable_interrupt();
        gpiote
            .channel1()
            .input_pin(&btn)
            .hi_to_lo()
            .enable_interrupt();

        (
            init::LateResources {
                gpiote,
                btn,
                echo_pin,
                trig_pin,
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

    #[task(binds = GPIOTE, resources = [gpiote])]
    fn on_gpiote(mut ctx: on_gpiote::Context) {
        ctx.resources.gpiote.lock(|gpiote| {
            if gpiote.channel0().is_event_triggered() {
                // echo_pin toggle event triggered the interrupt
                gpiote.reset_events();
                on_echo_toggle::spawn().ok();
            } else {
                // btn hi_to_lo event triggered the interrupt
                gpiote.reset_events();
                debounce::spawn_after(Milliseconds(30_u32)).ok();
            }
        });
    }

    #[task(resources = [echo_pin])]
    fn on_echo_toggle(mut ctx: on_echo_toggle::Context) {
        static mut START: Option<Instant<MyMono>> = None;
        if ctx.resources.echo_pin.lock(|pin| pin.is_high().unwrap()) {
            // Echo pulse started - store the start time
            START.replace(monotonics::MyMono::now());
        } else {
            // Echo pulse ended - calculate pulse duration
            if let Some(instant) = START.take() {
                let diff: Option<Microseconds> = monotonics::MyMono::now()
                    .checked_duration_since(&instant)
                    .and_then(|dur| dur.try_into().ok());
                if let Some(Microseconds(t)) = diff {
                    defmt::info!("Distance: {} cm", t / 58);
                }
            }
        }
    }

    #[task(resources = [btn, trig_pin])]
    fn debounce(mut ctx: debounce::Context) {
        if ctx.resources.btn.lock(|btn| btn.is_low().unwrap()) {
            // Button was pressed - send wave
            ctx.resources.trig_pin.lock(|pin| {
                pin.set_high().ok();
                cortex_m::asm::delay(640);
                pin.set_low().ok();
            });
        }
    }
}
