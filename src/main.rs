#![no_main]
#![no_std]
#![allow(non_snake_case)]

use panic_itm as _;

use cortex_m::asm::delay;
use rtic::{app, cyccnt::U32Ext, Exclusive, Mutex};
use stm32f3xx_hal::{
    prelude::*,
    gpio::{gpioe::PE9, Output, PushPull},
    usb::{Peripheral, UsbBus, UsbBusType},
};
use usb_device::bus;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use rtic::cyccnt::{Instant, Duration};


const CYCLES_PER_MILLISECOND: u32 = 72_000;
const CYCLES_PER_MICROSECOND: u32 = (CYCLES_PER_MILLISECOND as f64 / 1000f64) as u32;
const BLINK_ON_DURATION: u32 = 600 * CYCLES_PER_MILLISECOND;
const BLINK_OFF_DURATION: u32 = 200 * CYCLES_PER_MILLISECOND;
const WRITE1_PERIOD: u32 = 400 * CYCLES_PER_MILLISECOND;
const WRITE2_PERIOD: u32 = 399 * CYCLES_PER_MILLISECOND;


#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        led: PE9<Output<PushPull>>,
        usb_dev: UsbDevice<'static, UsbBusType>,
        serial: SerialPort<'static, UsbBusType>,
    }

    #[init(schedule = [blinker])]
    fn init(mut cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBusType>> = None;

        // enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        let mut flash = cx.device.FLASH.constrain();
        let mut rcc = cx.device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(72.MHz())
            .pclk1(24.MHz())
            .pclk2(24.MHz())
            .freeze(&mut flash.acr);

        assert!(clocks.usbclk_valid());

        // configure led
        let mut gpioe = cx.device.GPIOE.split(&mut rcc.ahb);
        let mut led = gpioe
            .pe9
            .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
        led.set_high().ok(); // Turn off

        let mut gpioa = cx.device.GPIOA.split(&mut rcc.ahb);

        // Pull the D+ pin down to send a RESET condition to the USB bus.
        // This forced reset is needed only for development, without it host
        // will not reset your device when you upload new firmware.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
        usb_dp.set_low().ok();
        delay(CYCLES_PER_MILLISECOND);

        let usb_dm = gpioa.pa11.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
        let usb_dp = usb_dp.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);

        let usb = Peripheral {
            usb: cx.device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        unsafe { *USB_BUS = Some(UsbBus::new(usb)) };

        let serial = unsafe { SerialPort::new(USB_BUS.as_ref().unwrap()) };

        let usb_dev = unsafe { UsbDeviceBuilder::new(
            USB_BUS.as_ref().unwrap(),
            UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build() };

        cx.schedule.blinker(cx.start + BLINK_ON_DURATION.cycles()).unwrap();

        init::LateResources { led, usb_dev, serial }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
            cortex_m::asm::wfi();
        }
    }

    #[task(priority = 3, binds = USB_HP_CAN_TX, resources = [usb_dev, serial])]
    fn usb_tx(mut cx: usb_tx::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.serial);
    }

    #[task(priority = 3, binds = USB_LP_CAN_RX0, resources = [usb_dev, serial])]
    fn usb_rx0(mut cx: usb_rx0::Context) {
        usb_poll(&mut cx.resources.usb_dev, &mut cx.resources.serial);
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

    extern "C" {
        fn EXTI0();
    }
};

fn usb_poll<B: bus::UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    serial: &mut SerialPort<'static, B>,
) {
    if !usb_dev.poll(&mut [serial]){
        return;
    }

    static mut NEXT_WRITE1_TIME: Option<Instant> = None;
    static mut NEXT_WRITE2_TIME: Option<Instant> = None;

    unsafe {
        match (NEXT_WRITE1_TIME, NEXT_WRITE2_TIME) {
            (None, None) => {
                NEXT_WRITE1_TIME = Some(Instant::now());
                NEXT_WRITE2_TIME = Some(Instant::now());
            },
            _ => {}
        }

        scheduled_write(NEXT_WRITE1_TIME.as_mut().unwrap(), WRITE1_PERIOD.cycles(), "Hello!\n\r", serial);
        scheduled_write(NEXT_WRITE2_TIME.as_mut().unwrap(), WRITE2_PERIOD.cycles(), "World!\n\r", serial);
    }
}

fn scheduled_write<B: bus::UsbBus>(
    next_write_time: &mut Instant,
    period: Duration,
    message: &str,
    serial: &mut SerialPort<'static, B>
) {
    if Instant::now() - period >= *next_write_time {
        serial_write(serial, message.as_bytes());
        *next_write_time += period;
    }
}

fn serial_write<B: bus::UsbBus>(serial: &mut SerialPort<'static, B>, data: &[u8]) {
    let mut cur_offset = 0;

    while cur_offset < data.len() {
        match serial.write(&data[cur_offset..]) {
            Ok(count_written) => {
                cur_offset += count_written;
            }
            _ => {}
        };
    }
}