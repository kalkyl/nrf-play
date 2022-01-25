#![no_main]
#![no_std]

use nrf_play as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [UARTE1])]
mod app {
    use embedded_sdmmc::{TimeSource, Timestamp, VolumeIdx};
    use nrf52840_hal::{
        gpio::{p0::Parts, Level, Output, Pin, PushPull},
        pac::{SPIM2, TIMER0},
        prelude::*,
        Spim,
    };
    use nrf_play::mono::{ExtU32, MonoTimer};

    #[monotonic(binds = TIMER0, default = true)]
    type Monotonic = MonoTimer<TIMER0>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = MonoTimer::new(ctx.device.TIMER0);
        let p0 = Parts::new(ctx.device.P0);
        let led = p0.p0_13.into_push_pull_output(Level::High).degrade();

        let sdmmc_cs = p0.p0_26.into_push_pull_output(Level::High).degrade();
        let sdmmc_mosi = p0.p0_04.into_push_pull_output(Level::Low).degrade();
        let sdmmc_miso = p0.p0_06.into_floating_input().degrade();
        let sdmmc_clk = p0.p0_08.into_push_pull_output(Level::Low).degrade();

        let sdmmc_spi = Spim::new(
            ctx.device.SPIM2,
            nrf52840_hal::spim::Pins {
                sck: sdmmc_clk,
                miso: Some(sdmmc_miso),
                mosi: Some(sdmmc_mosi),
            },
            nrf52840_hal::spim::Frequency::M16,
            nrf52840_hal::spim::MODE_0,
            0,
        );

        let mut sdmmc_controller = embedded_sdmmc::Controller::new(
            embedded_sdmmc::SdMmcSpi::new(MySpim(sdmmc_spi), sdmmc_cs),
            SdMmcClock,
        );

        defmt::info!("Init SD card...\r");
        match sdmmc_controller.device().init() {
            Ok(_) => {
                defmt::info!("Card size... ");
                match sdmmc_controller.device().card_size_bytes() {
                    Ok(size) => defmt::info!("{}\r", size),
                    Err(e) => defmt::info!("Err: {:?}", e),
                }
                defmt::info!("Volume 0:\r");
                match sdmmc_controller.get_volume(VolumeIdx(0)) {
                    Ok(volume) => {
                        let root_dir = sdmmc_controller.open_root_dir(&volume).unwrap();
                        defmt::info!("Listing root directory:\r");
                        sdmmc_controller
                            .iterate_dir(&volume, &root_dir, |x| {
                                defmt::info!("Found: {:?}\r", x.name);
                            })
                            .unwrap();
                        defmt::info!("End of listing\r");
                    }
                    Err(e) => defmt::info!("Err: {:?}", e),
                }
            }
            Err(e) => defmt::info!("{:?}!", e),
        }

        blink::spawn().ok();
        (Shared {}, Local { led }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(local = [led])]
    fn blink(ctx: blink::Context) {
        defmt::info!("Blink!");
        let led = ctx.local.led;
        if led.is_set_low().unwrap() {
            led.set_high().ok();
        } else {
            led.set_low().ok();
        }
        blink::spawn_after(1_u32.secs()).ok();
    }

    pub struct SdMmcClock;

    impl TimeSource for SdMmcClock {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 0,
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            }
        }
    }

    pub struct MySpim(Spim<SPIM2>);

    impl embedded_hal::spi::FullDuplex<u8> for MySpim {
        type Error = nrf52840_hal::spim::Error;

        fn read(&mut self) -> nb::Result<u8, Self::Error> {
            let mut buf = [0; 1];
            <Spim<SPIM2> as embedded_hal::blocking::spi::Transfer<u8>>::transfer(
                &mut self.0,
                &mut buf,
            )?;
            Ok(buf[0])
        }

        fn send(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
            <Spim<SPIM2> as embedded_hal::blocking::spi::Write<u8>>::write(&mut self.0, &[byte])?;
            Ok(())
        }
    }
}
