use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use gilrs::{Axis, Button, Gilrs};
use multiinput::{RawEvent, RawInputManager};

use pad_motion::protocol::*;
use pad_motion::server::*;

/// Gamepad and mouse info server.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Invert mouse X-axis.
    #[arg(long)]
    invert_x: bool,

    /// Invert mouse Y-axis.
    #[arg(long)]
    invert_y: bool,

    /// Invert gamepad left stick X-axis.
    #[arg(long)]
    invert_gamepad_x: bool,

    /// Invert gamepad left stick Y-axis.
    #[arg(long)]
    invert_gamepad_y: bool,

    /// Invert gamepad right stick X-axis.
    #[arg(long)]
    invert_gamepad_right_x: bool,

    /// Invert gamepad right stick Y-axis.
    #[arg(long)]
    invert_gamepad_right_y: bool,
}

fn main() {
    let args = Args::parse();

    let running = Arc::new(AtomicBool::new(true));

    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    let server = Arc::new(Server::new(None, None).unwrap());
    let server_thread_join_handle = {
        let server = server.clone();
        server.start(running.clone())
    };

    let controller_info = ControllerInfo {
        slot_state: SlotState::Connected,
        device_type: DeviceType::FullGyro,
        connection_type: ConnectionType::USB,
        ..Default::default()
    };
    server.update_controller_info(controller_info);

    fn to_stick_value(input: f32) -> u8 {
        (input * 127.0 + 127.0) as u8
    }

    fn inverter(invert: bool) -> f32 {
        if invert {
            -1.0
        } else {
            1.0
        }
    }

    let inverter_x = inverter(args.invert_x);
    let inverter_y = inverter(args.invert_y);
    let gamepad_inverter_x = inverter(args.invert_gamepad_x);
    let gamepad_inverter_y = inverter(args.invert_gamepad_y);
    let gamepad_inverter_right_x = inverter(args.invert_gamepad_right_x);
    let gamepad_inverter_right_y = inverter(args.invert_gamepad_right_y);

    let mut gilrs = Gilrs::new().unwrap();
    let mut mouse_manager = RawInputManager::new().unwrap();
    mouse_manager.register_devices(multiinput::DeviceType::Mice);

    let now = Instant::now();
    while running.load(Ordering::SeqCst) {
        // Consume controller events
        while let Some(_event) = gilrs.next_event() {}

        let mut delta_rotation_x = 0.0;
        let mut delta_rotation_y = 0.0;
        let mut delta_mouse_wheel = 0.0;
        while let Some(event) = mouse_manager.get_event() {
            match event {
                RawEvent::MouseMoveEvent(_mouse_id, delta_x, delta_y) => {
                    delta_rotation_x += delta_x as f32;
                    delta_rotation_y += delta_y as f32;
                }
                RawEvent::MouseWheelEvent(_mouse_id, delta) => {
                    delta_mouse_wheel += delta as f32;
                }
                _ => (),
            }
        }

        let first_gamepad = gilrs.gamepads().next();
        let controller_data = {
            if let Some((_id, gamepad)) = first_gamepad {
                let analog_button_value = |button| {
                    gamepad
                        .button_data(button)
                        .map(|data| (data.value() * 255.0) as u8)
                        .unwrap_or(0)
                };

                ControllerData {
                    connected: true,
                    d_pad_left: gamepad.is_pressed(Button::DPadLeft),
                    d_pad_down: gamepad.is_pressed(Button::DPadDown),
                    d_pad_right: gamepad.is_pressed(Button::DPadRight),
                    d_pad_up: gamepad.is_pressed(Button::DPadUp),
                    start: gamepad.is_pressed(Button::Start),
                    right_stick_button: gamepad.is_pressed(Button::RightThumb),
                    left_stick_button: gamepad.is_pressed(Button::LeftThumb),
                    select: gamepad.is_pressed(Button::Select),
                    triangle: gamepad.is_pressed(Button::North),
                    circle: gamepad.is_pressed(Button::East),
                    cross: gamepad.is_pressed(Button::South),
                    square: gamepad.is_pressed(Button::West),
                    r1: gamepad.is_pressed(Button::RightTrigger),
                    l1: gamepad.is_pressed(Button::LeftTrigger),
                    r2: gamepad.is_pressed(Button::RightTrigger2),
                    l2: gamepad.is_pressed(Button::LeftTrigger2),
                    ps: analog_button_value(Button::Mode),
                    left_stick_x: to_stick_value(
                        gamepad.value(Axis::LeftStickX) * gamepad_inverter_x,
                    ),
                    left_stick_y: to_stick_value(
                        gamepad.value(Axis::LeftStickY) * gamepad_inverter_y,
                    ),
                    right_stick_x: to_stick_value(
                        gamepad.value(Axis::RightStickX) * gamepad_inverter_right_x,
                    ),
                    right_stick_y: to_stick_value(
                        gamepad.value(Axis::RightStickY) * gamepad_inverter_right_y,
                    ),
                    analog_d_pad_left: analog_button_value(Button::DPadLeft),
                    analog_d_pad_down: analog_button_value(Button::DPadDown),
                    analog_d_pad_right: analog_button_value(Button::DPadRight),
                    analog_d_pad_up: analog_button_value(Button::DPadUp),
                    analog_triangle: analog_button_value(Button::North),
                    analog_circle: analog_button_value(Button::East),
                    analog_cross: analog_button_value(Button::South),
                    analog_square: analog_button_value(Button::West),
                    analog_r1: analog_button_value(Button::RightTrigger),
                    analog_l1: analog_button_value(Button::LeftTrigger),
                    analog_r2: analog_button_value(Button::RightTrigger2),
                    analog_l2: analog_button_value(Button::LeftTrigger2),
                    motion_data_timestamp: now.elapsed().as_micros() as u64,
                    gyroscope_pitch: -delta_rotation_y * 10.0 * inverter_y,
                    gyroscope_roll: delta_rotation_x * 10.0 * inverter_x,
                    gyroscope_yaw: delta_mouse_wheel * 300.0,
                    ..Default::default()
                }
            } else {
                ControllerData {
                    connected: true,
                    motion_data_timestamp: now.elapsed().as_micros() as u64,
                    gyroscope_pitch: -delta_rotation_y * 10.0 * inverter_y,
                    gyroscope_roll: delta_rotation_x * 10.0 * inverter_x,
                    gyroscope_yaw: delta_mouse_wheel * 300.0,
                    ..Default::default()
                }
            }
        };

        server.update_controller_data(0, controller_data);

        std::thread::sleep(Duration::from_millis(10));
    }

    server_thread_join_handle.join().unwrap();
}
