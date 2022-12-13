use sdl2::{
    event::{Event, EventPollIterator},
    keyboard::Scancode,
    mouse::MouseButton,
};

pub struct InputProcessor {
    // TODO: we need to store something like this in order to handle mouse dragging with a key pressed
    // etc.
    // key_pressed: Option<Scancode>,
    // mouse_button_pressed: Option<MouseButton>,
    mouse_position: MousePosition,
}

impl InputProcessor {
    pub fn new() -> InputProcessor {
        InputProcessor {
            mouse_position: MousePosition::new(0, 0),
        }
    }

    pub fn process_frame_events(&mut self, iterator: EventPollIterator) -> InputProcessorResult {
        let mut key_pressed: Option<Scancode> = None;
        let mut mouse_button_pressed: Option<MouseButton> = None;
        for event in iterator {
            match event {
                Event::Quit { .. } => return InputProcessorResult::Quit,
                Event::KeyDown { scancode, .. } => {
                    key_pressed = scancode;
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    mouse_button_pressed = Some(mouse_btn);
                }
                Event::MouseMotion { x, y, .. } => {
                    // We currently have to divide the coordinates by two, because we
                    // scale the screen to be double the game's original resolution.
                    self.mouse_position = MousePosition::new(x as usize / 2, y as usize / 2);
                }
                _ => (),
            }
        }
        InputProcessorResult::Input(Input {
            mouse_position: self.mouse_position,
            key_pressed,
            mouse_button_pressed,
        })
    }
}

pub enum InputProcessorResult {
    Quit,
    Input(Input),
}

pub struct Input {
    pub mouse_position: MousePosition,
    pub key_pressed: Option<Scancode>,
    pub mouse_button_pressed: Option<MouseButton>,
}

#[derive(Copy, Clone)]
pub struct MousePosition {
    pub x: usize,
    pub y: usize,
}

impl MousePosition {
    pub fn new(x: usize, y: usize) -> MousePosition {
        // TODO: Check the range
        MousePosition { x, y }
    }
}
