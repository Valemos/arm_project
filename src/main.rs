#![no_main]
#![no_std]
#![deny(unused_imports)]

use panic_itm as _;
use rtic::{app, Exclusive, Mutex};

use cortex_m;
use stm32f3xx_hal as hal;

use hal::{
    prelude::*,
    gpio::{
        gpioe::PE9,
        Output, PushPull
    },
    usb::{Peripheral, UsbBus}
};
use rtic::cyccnt::U32Ext;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use usb_device::bus::UsbBusAllocator;


const CYCLES_PER_MILLISECOND: u32 = 72_000;
const BLINK_ON_DURATION: u32 = 600 * CYCLES_PER_MILLISECOND;
const BLINK_OFF_DURATION: u32 = 200 * CYCLES_PER_MILLISECOND;
const WRITE1_PERIOD: u32 = 400 * CYCLES_PER_MILLISECOND;
const WRITE2_PERIOD: u32 = 399 * CYCLES_PER_MILLISECOND;


static mut USB_BUS: Option<UsbBusAllocator<UsbBus<Peripheral>>> = None;

#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        led: PE9<Output<PushPull>>,
        serial_port: SerialPort<'static, UsbBus<Peripheral>>,
        usb_device: UsbDevice<'static, UsbBus<Peripheral>>,
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
        let _clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(72.MHz())
            .pclk1(36.MHz())
            .freeze(&mut flash.acr);

        // Setup LED
        let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
        let led = gpioe.pe9.into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);

        // Setup usb
        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
        let usb_dm = gpioa.pa11.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        let usb_dp = gpioa.pa12.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);

        let usb = Peripheral {
            usb: dp.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };


        let (serial_port, usb_device) = unsafe {
            USB_BUS = Some(UsbBus::new(usb));
            (
                SerialPort::new(&USB_BUS.as_ref().unwrap()),
                UsbDeviceBuilder::new(&USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
                    .manufacturer("Our company")
                    .product("Serial port")
                    .serial_number("TEST")
                    .device_class(USB_CLASS_CDC)
                    .build()
            )
        };

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + BLINK_ON_DURATION.cycles()).unwrap();

        // Schedule first write task
        cx.schedule.write_hello(cx.start + (100 * CYCLES_PER_MILLISECOND).cycles()).unwrap();

        // Schedule second write task
        cx.schedule.write_world(cx.start + (100 * CYCLES_PER_MILLISECOND).cycles()).unwrap();

        init::LateResources {
            led,
            serial_port,
            usb_device,
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

    #[task(priority = 1, resources = [usb_device, serial_port], schedule = [write_hello])]
    fn write_hello(cx: write_hello::Context) {
        write_to_usb(cx.resources.usb_device, cx.resources.serial_port, "Hello\n\r");
        cx.schedule.write_hello(cx.scheduled + WRITE1_PERIOD.cycles()).unwrap()
    }

    #[task(priority = 1, resources = [usb_device, serial_port], schedule = [write_world])]
    fn write_world(cx: write_world::Context) {
        write_to_usb(cx.resources.usb_device, cx.resources.serial_port, "World\n\r");
        cx.schedule.write_world(cx.scheduled + WRITE2_PERIOD.cycles()).unwrap()
    }

    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
    }
};

#[inline(always)]
fn write_to_usb(usb_device: &mut UsbDevice<UsbBus<Peripheral>>, serial_port: &mut SerialPort<UsbBus<Peripheral>>, string: &str) {
    Exclusive(usb_device).lock(|usb_device| {
        if usb_device.poll(&mut [serial_port]) {
            cortex_m::asm::bkpt();
            serial_port.write(string.as_bytes()).unwrap();
        } else {
            cortex_m::asm::nop();
            cortex_m::asm::delay(10 * CYCLES_PER_MILLISECOND);
        }
    });
}
