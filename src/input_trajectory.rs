use std::sync::Mutex;

use bevy::{input::gamepad::GamepadAxisChangedEvent, prelude::*};

pub struct InputTrajectory;

impl Plugin for InputTrajectory {
    fn build(&self, app: &mut App) {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct InputVec2 {
    x: f32,
    y: f32,
}

pub static FPS_20_INPUT_VEC2: Mutex<InputVec2> = Mutex::new(InputVec2 { x: 0.0, y: 0.0 });
pub static FPS_40_INPUT_VEC2: Mutex<InputVec2> = Mutex::new(InputVec2 { x: 0.0, y: 0.0 });
pub static FPS_60_INPUT_VEC2: Mutex<InputVec2> = Mutex::new(InputVec2 { x: 0.0, y: 0.0 });

pub static FPS_20_INPUT_PREDICTIONS: Mutex<Vec<InputVec2>> = Mutex::new(Vec::new());
pub static FPS_40_INPUT_PREDICTIONS: Mutex<Vec<InputVec2>> = Mutex::new(Vec::new());
pub static FPS_60_INPUT_PREDICTIONS: Mutex<Vec<InputVec2>> = Mutex::new(Vec::new());

pub fn calc_fps_delay_duration_ms(fps: u64) -> u64 {
    // assuming bevy runs normally on 60fps
    return 1 / 60 * 1000 * (60 - fps);
}

// for now movement only consider the left joystick
// not too sure about the implementations for the 40fps and 20fps versions (i assume bevy runs normally on 60fps)
// reference: https://github.com/bevyengine/bevy/issues/1343#issuecomment-997513775
pub fn movement_events_60fps(mut axis_changed_events: EventReader<GamepadAxisChangedEvent>) {
    let mut input_predictions = FPS_60_INPUT_PREDICTIONS.lock().unwrap();
    let mut input_vec2 = FPS_60_INPUT_VEC2.lock().unwrap();

    for axis_changed_event in axis_changed_events.read() {
        log_movement_event(axis_changed_event);

        if axis_changed_event.axis_type == GamepadAxisType::LeftStickX {
            // x-axis position value
            input_vec2.x = axis_changed_event.value;
        }
        if axis_changed_event.axis_type == GamepadAxisType::LeftStickY {
            // y-axis position value
            input_vec2.y = axis_changed_event.value;
        }

        let cloned_input_vec2 = input_vec2.clone();
        if input_predictions.len() > 5 {
            let overflow_count = input_predictions.len() - 5;
            for _ in 0..overflow_count {
                input_predictions.remove(0);
            }
        }
        input_predictions.push(cloned_input_vec2);

        println!("===============Movement Prediction (for 60fps)======================");
        for el in &*input_predictions {
            println!("x: {}; y: {}", el.x, el.y);
        }
        println!("====================================================================");
    }
}

pub fn movement_events_40fps(mut axis_changed_events: EventReader<GamepadAxisChangedEvent>) {
    let mut input_predictions = FPS_40_INPUT_PREDICTIONS.lock().unwrap();
    let mut input_vec2 = FPS_40_INPUT_VEC2.lock().unwrap();
    use std::{thread, time};

    for axis_changed_event in axis_changed_events.read() {
        log_movement_event(axis_changed_event);

        if axis_changed_event.axis_type == GamepadAxisType::LeftStickX {
            input_vec2.x = axis_changed_event.value;
        }
        if axis_changed_event.axis_type == GamepadAxisType::LeftStickY {
            input_vec2.y = axis_changed_event.value;
        }

        let cloned_input_vec2 = input_vec2.clone();
        if input_predictions.len() > 5 {
            let overflow_count = input_predictions.len() - 5;
            for _ in 0..overflow_count {
                input_predictions.remove(0);
            }
        }
        input_predictions.push(cloned_input_vec2);

        println!("===============Movement Prediction (for 40fps)======================");
        for el in &*input_predictions {
            println!("x: {}; y: {}", el.x, el.y);
        }
        println!("====================================================================");

        thread::sleep(time::Duration::from_millis(calc_fps_delay_duration_ms(40)));
    }
}

pub fn movement_events_20fps(mut axis_changed_events: EventReader<GamepadAxisChangedEvent>) {
    let mut input_predictions = FPS_20_INPUT_PREDICTIONS.lock().unwrap();
    let mut input_vec2 = FPS_20_INPUT_VEC2.lock().unwrap();
    use std::{thread, time};

    for axis_changed_event in axis_changed_events.read() {
        log_movement_event(axis_changed_event);

        if axis_changed_event.axis_type == GamepadAxisType::LeftStickX {
            input_vec2.x = axis_changed_event.value;
        }
        if axis_changed_event.axis_type == GamepadAxisType::LeftStickY {
            input_vec2.y = axis_changed_event.value;
        }

        let cloned_input_vec2 = input_vec2.clone();
        if input_predictions.len() > 5 {
            let overflow_count = input_predictions.len() - 5;
            for _ in 0..overflow_count {
                input_predictions.remove(0);
            }
        }
        input_predictions.push(cloned_input_vec2);

        println!("===============Movement Prediction (for 20fps)======================");
        for el in &*input_predictions {
            println!("x: {}; y: {}", el.x, el.y);
        }
        println!("====================================================================");

        thread::sleep(time::Duration::from_millis(calc_fps_delay_duration_ms(20)));
    }
}

pub fn log_movement_event(axis_changed_event: &GamepadAxisChangedEvent) {
    info!(
        "{:?} of {:?} is changed to {}",
        axis_changed_event.axis_type, axis_changed_event.gamepad, axis_changed_event.value
    );
}
