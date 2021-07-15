#![no_main]
#![no_std]

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
const CYCLES_PER_MICROSECOND: u32 = 72;
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
        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(72.MHz())
            .pclk1(24.MHz())
            .pclk2(24.MHz())
            .freeze(&mut flash.acr);

        // Setup LED
        let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
        let led = gpioe.pe9.into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);

        // Setup usb
        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

        // send RESET condition to host
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
        usb_dp.set_low().ok();
        cortex_m::asm::delay(CYCLES_PER_MICROSECOND);

        // initialize usb pin modes
        let usb_dm = gpioa.pa11.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        let usb_dp = usb_dp.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);


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
        cx.schedule.write_hello(cx.start).unwrap();

        // Schedule second write task
        cx.schedule.write_world(cx.start).unwrap();

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

    #[task(priority = 2, resources = [usb_device, serial_port], schedule = [usb_poll])]
    fn usb_poll(cx: usb_poll::Context) {
        let usb_device: &mut UsbDevice<UsbBus<Peripheral>> = cx.resources.usb_device;
        let serial_port: &mut SerialPort<UsbBus<Peripheral>> = cx.resources.serial_port;

        usb_device.poll(&mut [serial_port]);
        cx.schedule.usb_poll(cx.scheduled + CYCLES_PER_MICROSECOND.cycles());
    }

    #[task(priority = 1, resources = [usb_device, serial_port], schedule = [write_hello])]
    fn write_hello(cx: write_hello::Context) {
        let usb_device: Exclusive<UsbDevice<UsbBus<Peripheral>>> = Exclusive(cx.resources.usb_device);
        while usb_device.state() != UsbDeviceState::Default {}

        write_to_usb(Exclusive(cx.resources.serial_port), "Hello\n\r");

        cx.schedule.write_hello(cx.scheduled + WRITE1_PERIOD.cycles()).unwrap()
    }

    #[task(priority = 1, resources = [usb_device, serial_port], schedule = [write_world])]
    fn write_world(cx: write_world::Context) {
        let usb_device: Exclusive<UsbDevice<UsbBus<Peripheral>>> = Exclusive(cx.resources.usb_device);
        while usb_device.state() != UsbDeviceState::Default {}

        write_to_usb(Exclusive(cx.resources.serial_port), "World\n\r");

        cx.schedule.write_world(cx.scheduled + WRITE2_PERIOD.cycles()).unwrap()
    }

    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
        fn EXTI3();
    }
};

#[inline(always)]
fn write_to_usb(mut serial_port: Exclusive<SerialPort<UsbBus<Peripheral>>>, string: &str) {
    let bytes = string.as_bytes();

    serial_port.lock(|serial_port|{
        let mut write_offset = 0;
        while write_offset < bytes.len() {
            match serial_port.write(&bytes[write_offset..bytes.len()]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                }
                _ => {}
            }
        }
    });
}
