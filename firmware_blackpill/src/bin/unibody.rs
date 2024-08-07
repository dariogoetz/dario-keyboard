#![no_main]
#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};
use defmt_rtt as _; // global logger
use frunk::{HCons, HNil};
use hal::gpio::{EPin, Input, Output, PushPull};
use hal::otg_fs::{UsbBusType, USB};
use hal::prelude::*;
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layout};
use keyberon::matrix::Matrix;
use stm32f4xx_hal as hal;
use synopsys_usb_otg::bus::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
use usb_device::prelude::*;
use usbd_human_interface_device::device::keyboard::{NKROBootKeyboard, NKROBootKeyboardConfig};
use usbd_human_interface_device::page::Keyboard;
use usbd_human_interface_device::usb_class::{UsbHidClass, UsbHidClassBuilder};

use panic_probe as _;

use dario_firmware_keyberon::layout;

/// USB VIP for a generic keyboard from
/// https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt
const VID: u16 = 0x16c0;

/// USB PID for a generic keyboard from
/// https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt
const PID: u16 = 0x27db;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", {
    // NOTE(no-CAS) `timestamps` runs with interrupts disabled
    let n = COUNT.load(Ordering::Relaxed);
    COUNT.store(n + 1, Ordering::Relaxed);
    n
});

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

type UsbClass =
    UsbHidClass<'static, UsbBus<USB>, HCons<NKROBootKeyboard<'static, UsbBus<USB>>, HNil>>;
type UsbDevice = usb_device::device::UsbDevice<'static, UsbBusType>;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers=[TIM1_CC])]
mod app {
    use super::*;
    // shared resources (between tasks)
    #[shared]
    struct Shared {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        #[lock_free]
        layout: Layout<12, 4, 5, ()>,
    }

    // local resources (between tasks)
    #[local]
    struct Local {
        matrix: Matrix<EPin<Input>, EPin<Output<PushPull>>, 12, 4>,
        debouncer: Debouncer<[[bool; 12]; 4]>,
        timer: hal::timer::counter::CounterHz<hal::pac::TIM2>,
        delay: cortex_m::delay::Delay,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init");
        // prepare static datastructures for USB
        static mut EP_MEMORY: [u32; 1024] = [0; 1024];
        static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

        // setup the monotonic timer
        let clocks = cx
            .device
            .RCC
            .constrain()
            .cfgr
            .use_hse(25.MHz())
            .sysclk(84.MHz())
            .require_pll48clk()
            .freeze();

        // get GPIO pins
        let gpioa = cx.device.GPIOA.split();
        let gpiob = cx.device.GPIOB.split();

        // timer for processing keyboard events and sending a USB keyboard report
        let mut timer = cx.device.TIM2.counter_hz(&clocks);
        // or equivalently
        // let mut timer = hal::timer::Timer::new(cx.device.TIM2, &mut clocks).counter_hz();
        timer.start(1.kHz()).unwrap();
        timer.listen(hal::timer::Event::Update);

        // initialize USB
        let usb = USB {
            usb_global: cx.device.OTG_FS_GLOBAL,
            usb_device: cx.device.OTG_FS_DEVICE,
            usb_pwrclk: cx.device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate().into(),
            pin_dp: gpioa.pa12.into_alternate().into(),
            hclk: clocks.hclk(),
        };

        unsafe {
            USB_BUS = Some(UsbBusType::new(usb, &mut EP_MEMORY));
        }

        let usb_bus = unsafe { USB_BUS.as_ref().unwrap() };
        let usb_class = UsbHidClassBuilder::new()
            .add_device(NKROBootKeyboardConfig::default())
            .build(usb_bus);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(VID, PID))
            .manufacturer("Dario Götz")
            .product("Dario Götz's 42-key unibody keyboard")
            .serial_number(env!("CARGO_PKG_VERSION"))
            .build();

        // define pin to matrix relation (prepare outside of interrupt::free closure
        // due to gpioa/gpiob move)
        let cols = [
            // left hand
            gpiob.pb12.into_pull_up_input().erase(),
            gpiob.pb14.into_pull_up_input().erase(),
            gpiob.pb15.into_pull_up_input().erase(),
            gpioa.pa8.into_pull_up_input().erase(),
            gpioa.pa15.into_pull_up_input().erase(),
            gpiob.pb3.into_pull_up_input().erase(),
            // right hand
            gpioa.pa1.into_pull_up_input().erase(),
            gpioa.pa2.into_pull_up_input().erase(),
            gpioa.pa4.into_pull_up_input().erase(),
            gpioa.pa6.into_pull_up_input().erase(),
            gpioa.pa7.into_pull_up_input().erase(),
            gpiob.pb1.into_pull_up_input().erase(),
        ];

        let rows = [
            gpiob.pb5.into_push_pull_output().erase(),
            gpiob.pb6.into_push_pull_output().erase(),
            gpiob.pb7.into_push_pull_output().erase(),
            // thumb cluster
            gpiob.pb8.into_push_pull_output().erase(),
        ];

        let matrix = cortex_m::interrupt::free(move |_cs| Matrix::new(cols, rows));

        let delay = cortex_m::delay::Delay::new(cx.core.SYST, clocks.sysclk().to_Hz());

        let mut layout = Layout::new(&layout::LAYERS);
        layout.add_tri_layer((1, 2), 3);

        (
            Shared {
                // Initialization of shared resources go here
                usb_dev,
                usb_class,
                layout,
            },
            Local {
                // Initialization of local resources go here
                matrix: matrix.unwrap(),
                timer,
                debouncer: Debouncer::new([[false; 12]; 4], [[false; 12]; 4], 5),
                delay,
            },
            init::Monotonics(),
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    /// Register a key press/release event with the layout (it will not be processed, yet)
    #[task(priority=1, capacity=8, shared=[layout])]
    fn register_keyboard_event(cx: register_keyboard_event::Context, event: Event) {
        // match event {
        //     Event::Press(i, j) => defmt::info!("Registering press {} {}", i, j),
        //     Event::Release(i, j) => defmt::info!("Registering release {} {}", i, j),
        // }
        cx.shared.layout.event(event)
    }

    /// Check all switches for their state, register corresponding events, and
    /// spawn generation of a USB keyboard report (including layout event processing)
    #[task(binds=TIM2, priority=1, local=[debouncer, matrix, timer, delay], shared=[usb_dev, usb_class, layout])]
    fn tick(mut cx: tick::Context) {
        // defmt::info!("Processing keyboard events");

        let delay = cx.local.delay;

        cx.local.timer.wait().ok();
        // or equivalently
        // cx.local.timer.clear_interrupt(hal::timer::Event::Update);

        // scan keyboard
        for event in cx.local.debouncer.events(
            cx.local
                .matrix
                .get_with_delay(|| delay.delay_us(10))
                .unwrap(),
        ) {
            cx.shared.layout.event(event)
            // match event {
            //     Event::Press(i, j) => defmt::info!("Pressed {} {}", i, j),
            //     Event::Release(i, j) => defmt::info!("Released {} {}", i, j),
            // }
        }

        let tick = cx.shared.layout.tick();
        if let CustomEvent::Release(()) = tick {
            unsafe { cortex_m::asm::bootload(0x1FFF0000 as _) }
        };

        // send a USB keyboard report
        while let Ok(()) = cx.shared.usb_class.lock(|k| {
            k.device()
                .write_report(cx.shared.layout.keycodes().map(|k| Keyboard::from(k as u8)))
        }) {}
    }

    // USB events
    #[task(binds = OTG_FS, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_tx(cx: usb_tx::Context) {
        (cx.shared.usb_dev, cx.shared.usb_class).lock(|usb_dev, usb_class| {
            usb_poll(usb_dev, usb_class);
        });
    }

    #[task(binds = OTG_FS_WKUP, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(cx: usb_rx::Context) {
        (cx.shared.usb_dev, cx.shared.usb_class).lock(|usb_dev, usb_class| {
            usb_poll(usb_dev, usb_class);
        });
    }

    fn usb_poll(usb_dev: &mut UsbDevice, keyboard: &mut UsbClass) {
        if usb_dev.poll(&mut [keyboard]) {
            keyboard.poll();
        }
    }
}
