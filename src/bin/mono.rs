use nrf52840_hal::pac::{timer0, TIMER0, TIMER1, TIMER2};
use rtic_monotonic::{embedded_time, Clock, Fraction, Instant, Monotonic};

pub struct MonoTimer<T: Instance32>(T);

impl<T: Instance32> MonoTimer<T> {
    pub fn new(timer: T) -> Self {
        timer.prescaler.write(
            |w| unsafe { w.prescaler().bits(4) }, // 1 MHz
        );
        timer.bitmode.write(|w| w.bitmode()._32bit());
        MonoTimer(timer)
    }
}

impl<T: Instance32> Clock for MonoTimer<T> {
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);
    type T = u32;

    #[inline(always)]
    fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
        self.0.tasks_capture[1].write(|w| unsafe { w.bits(1) });
        Ok(Instant::new(self.0.cc[1].read().bits()))
    }
}

impl<T: Instance32> Monotonic for MonoTimer<T> {
    unsafe fn reset(&mut self) {
        self.0.intenset.modify(|_, w| w.compare0().set());
        self.0.tasks_clear.write(|w| w.bits(1));
        self.0.tasks_start.write(|w| w.bits(1));
    }

    fn set_compare(&mut self, instant: &Instant<Self>) {
        self.0.cc[0].write(|w| unsafe { w.cc().bits(instant.duration_since_epoch().integer()) });
    }

    fn clear_compare_flag(&mut self) {
        self.0.events_compare[0].write(|w| w);
    }
}

pub trait Instance32: core::ops::Deref<Target = timer0::RegisterBlock> {}
impl Instance32 for TIMER0 {}
impl Instance32 for TIMER1 {}
impl Instance32 for TIMER2 {}
