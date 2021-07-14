#![no_main]
#![no_std]
#![deny(unused_imports)]

use panic_itm as _;
use rtic::{app, Exclusive};
use stm32f3xx_hal::{
    gpio::{
        gpioe::PE9,
        gpioc::{PC4, PC5},
        Alternate, Output, PushPull
    },
    serial::Serial,
    prelude::*,
    pac::USART1
};
use rtic::cyccnt::U32Ext;


const CYCLES_PER_MILLISECOND: u32 = 72_000;
const BLINK_ON_DURATION: u32 = 600 * CYCLES_PER_MILLISECOND;
const BLINK_OFF_DURATION: u32 = 200 * CYCLES_PER_MILLISECOND;
const WRITE1_PERIOD: u32 = 400 * CYCLES_PER_MILLISECOND;
const WRITE2_PERIOD: u32 = 399 * CYCLES_PER_MILLISECOND;


type SerialBus = Serial<USART1, (PC4<Alternate<PushPull, 7_u8>>, PC5<Alternate<PushPull, 7_u8>>)>;

#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        led: PE9<Output<PushPull>>,
        serial_port: SerialBus,
    }

    #[init(schedule = [blinker, write_hello, write_world])]
    fn init(cx: init::Context) -> init::LateResources {
        // Enable cycle counter
        let mut core = cx.core;
        core.DWT.enable_cycle_counter();

        let dp: stm32f3xx_hal::pac::Peripherals = cx.device;

        // Setup clocks
        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(72.MHz())
            .pclk1(36.MHz())
            .freeze(&mut flash.acr);

        // Setup LED
        let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
        let mut led = gpioe.pe9.into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);

        // Setup serial port
        let mut gpioc = dp.GPIOC.split(&mut rcc.ahb);

        let tx = gpioc.pc4.into_af7_push_pull(&mut gpioc.moder, &mut gpioc.otyper, &mut gpioc.afrl);
        let rx = gpioc.pc5.into_af7_push_pull(&mut gpioc.moder, &mut gpioc.otyper, &mut gpioc.afrl);

        let serial_port = Serial::new(dp.USART1, (tx, rx), 115_200.Bd(), clocks, &mut rcc.apb2);

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + BLINK_ON_DURATION.cycles()).unwrap();

        // Schedule first write task
        cx.schedule.write_hello(cx.start).unwrap();

        // Schedule second write task
        cx.schedule.write_world(cx.start).unwrap();

        init::LateResources {
            led,
            serial_port,
        }
    }

    #[task(priority = 2, resources = [led], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTIC
        static mut LED_ON_STATE: bool = false;

        unsafe {
            if *LED_ON_STATE {
                cx.resources.led.set_low().unwrap();
                *LED_ON_STATE = false;
                cx.schedule.blinker(cx.scheduled + BLINK_OFF_DURATION.cycles()).unwrap();
            } else {
                cx.resources.led.set_high().unwrap();
                *LED_ON_STATE = true;
                cx.schedule.blinker(cx.scheduled + BLINK_ON_DURATION.cycles()).unwrap();
            }
        }
    }

    #[task(priority = 1, resources = [serial_port], schedule = [write_hello])]
    fn write_hello(cx: write_hello::Context) {
        Exclusive(cx.resources.serial_port).lock(|serial| {
            write_to_serial(serial, "Hello\n\r");
        });
        cx.schedule.write_hello(cx.scheduled + WRITE1_PERIOD.cycles()).unwrap()
    }

    #[task(priority = 1, resources = [serial_port], schedule = [write_world])]
    fn write_world(cx: write_world::Context) {
        Exclusive(cx.resources.serial_port).lock(|serial| {
            write_to_serial(serial, "World\n\r");
        });
        cx.schedule.write_world(cx.scheduled + WRITE2_PERIOD.cycles()).unwrap()
    }

    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
    }
};


fn write_to_serial(serial: &mut SerialBus, string: &str) {
    for byte in string.bytes() {
        while !serial.is_txe() {}
        serial.write(byte).ok();
    }
}
