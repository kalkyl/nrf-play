#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Input, Level, Output, Pin, PullUp, PushPull},
        gpiote::Gpiote,
        pac::TIMER0,
        ppi,
        prelude::*,
        timer::Timer,
    };
    use rtic::time::duration::Milliseconds;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        gpiote: Gpiote,
        btn: Pin<Input<PullUp>>,
        trig_pin: Pin<Output<PushPull>>,
        timer: Timer<TIMER0>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let trig_pin = p0.p0_03.into_push_pull_output(Level::Low).degrade();
        let echo_pin = p0.p0_04.into_pulldown_input().degrade();
        let btn = p0.p0_11.into_pullup_input().degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote.channel0().input_pin(&echo_pin).lo_to_hi();
        gpiote
            .channel1()
            .input_pin(&echo_pin)
            .hi_to_lo()
            .enable_interrupt();
        gpiote
            .channel2()
            .input_pin(&btn)
            .hi_to_lo()
            .enable_interrupt();

        let timer = Timer::new(ctx.device.TIMER0);

        let mut ppi = ppi::Parts::new(ctx.device.PPI);
        ppi.ppi0.set_event_endpoint(gpiote.channel0().event());
        ppi.ppi0.set_task_endpoint(timer.task_start());
        ppi.ppi0.set_fork_task_endpoint(timer.task_clear());
        ppi.ppi0.enable();
        ppi.ppi1.set_event_endpoint(gpiote.channel1().event());
        ppi.ppi1.set_task_endpoint(timer.task_stop());
        ppi.ppi1.enable();

        (
            Shared {},
            Local {
                gpiote,
                btn,
                trig_pin,
                timer,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(binds = GPIOTE, local = [gpiote, timer])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        let (gpiote, timer) = (ctx.local.gpiote, ctx.local.timer);
        if gpiote.channel1().is_event_triggered() {
            // Echo pulse end triggered the interrupt
            gpiote.reset_events();
            defmt::info!("Distance: {} cm", timer.read() as f32 / 58.0);
        } else {
            // Button hi_to_low triggered the interrupt
            gpiote.reset_events();
            debounce::spawn_after(Milliseconds(30_u32)).ok();
        }
    }

    #[task(local = [btn, trig_pin])]
    fn debounce(ctx: debounce::Context) {
        if ctx.local.btn.is_low().unwrap() {
            // Button is pressed - send wave
            ctx.local.trig_pin.set_high().ok();
            cortex_m::asm::delay(640); // 10us
            ctx.local.trig_pin.set_low().ok();
        }
    }
}
