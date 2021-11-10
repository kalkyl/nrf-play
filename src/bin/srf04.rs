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
        gpio::{p0::Parts, Input, Level, Output, Pin, PullDown, PushPull},
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
        echo_pin: Pin<Input<PullDown>>,
        trig_pin: Pin<Output<PushPull>>,
        gpiote: Gpiote,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, FREQ);

        let p0 = Parts::new(ctx.device.P0);
        let echo_pin = p0.p0_04.into_pulldown_input().degrade();
        let trig_pin = p0.p0_03.into_push_pull_output(Level::Low).degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote
            .channel0()
            .input_pin(&echo_pin)
            .toggle() // Trigger on both rising and falling edges
            .enable_interrupt();

        send_wave::spawn().ok();

        (
            Shared {},
            Local {
                echo_pin,
                trig_pin,
                gpiote,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(local = [trig_pin])]
    fn send_wave(ctx: send_wave::Context) {
        ctx.local.trig_pin.set_high().ok();
        cortex_m::asm::delay(640); //10us
        ctx.local.trig_pin.set_low().ok();
        send_wave::spawn_after(100.millis()).ok();
    }

    #[task(binds = GPIOTE, local = [gpiote, echo_pin, start: Option<TimerInstantU32<FREQ>> = None])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        ctx.local.gpiote.reset_events();
        if ctx.local.echo_pin.is_high().unwrap() {
            // Echo pulse started - store start time
            ctx.local.start.replace(monotonics::now());
        } else {
            // Echo pulse ended - calculate pulse duration
            if let Some(instant) = ctx.local.start.take() {
                let t: MicrosDurationU32 = (monotonics::now() - instant).convert();
                defmt::info!("Distance: {} cm", t.ticks() as f32 / 58.0);
            }
        }
    }
}
