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
        gpio::{p0::Parts, Input, Level, Output, Pin, PullDown, PullUp, PushPull},
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
        gpiote: Gpiote,
        btn: Pin<Input<PullUp>>,
        echo_pin: Pin<Input<PullDown>>,
        trig_pin: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, FREQ);

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
            Shared {},
            Local {
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
        loop {}
    }

    #[task(binds = GPIOTE, local = [gpiote])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        let gpiote = ctx.local.gpiote;
        if gpiote.channel0().is_event_triggered() {
            // echo_pin toggle event triggered the interrupt
            gpiote.reset_events();
            on_echo_toggle::spawn().ok();
        } else {
            // btn hi_to_lo event triggered the interrupt
            gpiote.reset_events();
            debounce::spawn_after(30.millis()).ok();
        }
    }

    #[task(local = [echo_pin, start: Option<TimerInstantU32<FREQ>> = None])]
    fn on_echo_toggle(ctx: on_echo_toggle::Context) {
        if ctx.local.echo_pin.is_high().unwrap() {
            // Echo pulse started - store the start time
            ctx.local.start.replace(monotonics::now());
        } else {
            // Echo pulse ended - calculate pulse duration
            if let Some(instant) = ctx.local.start.take() {
                let t: MicrosDurationU32 = (monotonics::now() - instant).convert();
                defmt::info!("Distance: {} cm", t.ticks() as f32 / 58.0);
            }
        }
    }

    #[task(local = [btn, trig_pin])]
    fn debounce(ctx: debounce::Context) {
        if ctx.local.btn.is_low().unwrap() {
            // Button is pressed - send wave
            ctx.local.trig_pin.set_high().ok();
            cortex_m::asm::delay(640);
            ctx.local.trig_pin.set_low().ok();
        }
    }
}
