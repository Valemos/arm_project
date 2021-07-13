#![no_main]
#![no_std]

use panic_itm as _;
use rtic::app;
use stm32f3xx_hal::{
    gpio::{
        gpioe::PE9,
        gpioc::{PC4, PC5},
        Alternate, Input, Output, PushPull
    },
    serial::Serial,
    prelude::*,
    pac::USART1
};
use rtic::cyccnt::U32Ext;


// time in milliseconds
const BLINK_ON_DURATION: u32 = 600;
const BLINK_OFF_DURATION: u32 = 200;
const WRITE1_PERIOD: u32 = 400;
const WRITE2_PERIOD: u32 = 399;


#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        led: PE9<Output<PushPull>>,
        serial_port: Serial<USART1, (PC4<Alternate<PushPull, 7_u8>>, PC5<Alternate<PushPull, 7_u8>>)>,
    }

    #[init(schedule = [blinker])]
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
        led.set_high().unwrap();

        // Setup serial port
        let mut gpioc = dp.GPIOC.split(&mut rcc.ahb);

        let tx = gpioc.pc4.into_af7_push_pull(&mut gpioc.moder, &mut gpioc.otyper, &mut gpioc.afrl);
        let rx = gpioc.pc5.into_af7_push_pull(&mut gpioc.moder, &mut gpioc.otyper, &mut gpioc.afrl);

        let serial_port = Serial::new(dp.USART1, (tx, rx), 115_200.Bd(), clocks, &mut rcc.apb2);

        // Schedule the blinking task
        // cx.schedule.blinker(cx.start + BLINK_ON_DURATION.cycles()).unwrap();

        // Schedule first write task

        // Schedule second write task


        init::LateResources {
            led,
            serial_port,
        }
    }

    #[task(resources = [led], schedule = [blinker])]
    fn blinker(cx: blinker::Context) {
        // Use the safe local `static mut` of RTIC
        static mut LED_STATE: bool = false;

        unsafe {
            if *LED_STATE {
                cx.resources.led.set_high().unwrap();
                *LED_STATE = false;
            } else {
                cx.resources.led.set_low().unwrap();
                *LED_STATE = true;
            }
        }
        // cx.schedule.blinker(cx.scheduled + BLINK_ON_DURATION.cycles()).unwrap();
    }

    #[task(binds = USART1_EXTI25, resources = [serial_port])]
    fn write_hello(cx: write_hello::Context) {
        let mut serial_port = cx.resources.serial_port;

        let received = serial_port.read().unwrap();
        serial_port.write(received).ok();
    }

    extern "C" {
        fn EXTI0();
    }
};
