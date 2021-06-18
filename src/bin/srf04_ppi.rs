#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, peripherals = true, dispatchers = [UARTE1])]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use nrf52840_hal::{
        clocks::Clocks,
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        gpiote::Gpiote,
        pac::TIMER0,
        ppi,
        prelude::*,
        timer::Timer,
    };
    use rtic::time::duration::Milliseconds;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<64_000_000>; // 64 MHz

    #[resources]
    struct Resources {
        trig_pin: Pin<Output<PushPull>>,
        gpiote: Gpiote,
        timer: Timer<TIMER0>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let _clocks = Clocks::new(ctx.device.CLOCK).enable_ext_hfosc();

        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();
        let mono = DwtSystick::new(&mut ctx.core.DCB, ctx.core.DWT, ctx.core.SYST, 64_000_000);

        let p0 = Parts::new(ctx.device.P0);
        let echo_pin = p0.p0_04.into_pulldown_input().degrade();
        let trig_pin = p0.p0_03.into_push_pull_output(Level::Low).degrade();

        let gpiote = Gpiote::new(ctx.device.GPIOTE);
        gpiote.channel0().input_pin(&echo_pin).lo_to_hi();
        gpiote
            .channel1()
            .input_pin(&echo_pin)
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

        send_wave::spawn().ok();

        (
            init::LateResources {
                trig_pin,
                gpiote,
                timer,
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

    #[task(resources = [trig_pin])]
    fn send_wave(mut ctx: send_wave::Context) {
        ctx.resources.trig_pin.lock(|pin| {
            pin.set_high().ok();
            cortex_m::asm::delay(640); // 10us
            pin.set_low().ok();
        });
        send_wave::spawn_after(Milliseconds(100_u32)).ok();
    }

    #[task(binds = GPIOTE, resources = [gpiote, timer])]
    fn on_gpiote(mut ctx: on_gpiote::Context) {
        ctx.resources.gpiote.lock(|gpiote| gpiote.reset_events());
        ctx.resources
            .timer
            .lock(|timer| defmt::info!("Distance: {} cm", timer.read() as f32 / 58.0));
    }
}
