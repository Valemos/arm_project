#![no_main]
#![no_std]

use panic_itm as _;
use rtic::{app, Exclusive, Mutex};

use cortex_m;
use cortex_m::iprintln;
use cortex_m::peripheral::ITM;
use stm32f3xx_hal as hal;

use hal::{
    pac,
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
use cortex_m::peripheral::itm::Stim;


const CYCLES_PER_MILLISECOND: u32 = 48;
const CYCLES_PER_MICROSECOND: u32 = 48;
const BLINK_ON_DURATION: u32 = 600 * CYCLES_PER_MILLISECOND;
const BLINK_OFF_DURATION: u32 = 200 * CYCLES_PER_MILLISECOND;
const WRITE1_PERIOD: u32 = 400 * CYCLES_PER_MILLISECOND;
const WRITE2_PERIOD: u32 = 399 * CYCLES_PER_MILLISECOND;


static mut USB_BUS: Option<UsbBusAllocator<UsbBus<Peripheral>>> = None;

macro_rules! log {
    ($msg:expr) => {
        let itm = unsafe { &mut *ITM::ptr() };
        iprintln!(&mut itm.stim[0], $msg);
    }
}

#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        led: PE9<Output<PushPull>>,
        serial_port: SerialPort<'static, UsbBus<Peripheral>>,
        usb_device: UsbDevice<'static, UsbBus<Peripheral>>,
    }

    #[init(schedule = [blinker, write_hello])]
    fn init(cx: init::Context) -> init::LateResources {
        let dp = cx.device;

        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(48.MHz())
            .pclk1(24.MHz())
            .pclk2(24.MHz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        // Configure the on-board LED (LD10, south red)
        let mut gpioe = dp.GPIOE.split(&mut rcc.ahb);
        let mut led = gpioe
            .pe9
            .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
        led.set_low().ok(); // Turn off

        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

        // F3 Discovery board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        // This forced reset is needed only for development, without it host
        // will not reset a device
        let mut usb_dp = gpioa
            .pa12
            .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
        usb_dp.set_low().ok();
        cortex_m::asm::delay(CYCLES_PER_MILLISECOND);

        let usb_dm = gpioa.pa11.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        let usb_dp = usb_dp.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);

        let usb = Peripheral {
            usb: dp.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        unsafe { USB_BUS = Some(UsbBus::new(usb)) }

        let serial = unsafe { SerialPort::new(USB_BUS.as_ref().unwrap()) };

        let usb_dev = unsafe { UsbDeviceBuilder::new(
            USB_BUS.as_ref().unwrap(),
            UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build() };

        // Schedule the blinking task
        cx.schedule.blinker(cx.start + BLINK_ON_DURATION.cycles()).unwrap();

        // Schedule first write task
        cx.schedule.write_hello(cx.start + WRITE1_PERIOD.cycles()).unwrap();

        log!("init");

        init::LateResources {
            led,
            serial_port: serial,
            usb_device: usb_dev,
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

    #[task(priority = 1, resources = [usb_device, serial_port])]
    fn write_hello(cx: write_hello::Context) {
        let mut usb_device = cx.resources.usb_device;
        let mut serial_port = cx.resources.serial_port;

        loop {
            if !usb_device.poll(&mut [serial_port]) {
                continue;
            }

            write_serial_port(Exclusive(&mut serial_port), "Hello\n\r");
        }
    }

    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
        fn EXTI3();
    }
};

#[inline(always)]
fn write_serial_port(mut serial_port: Exclusive<SerialPort<UsbBus<Peripheral>>>, string: &str) {
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
