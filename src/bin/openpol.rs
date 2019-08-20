use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window(&format!("openpol {}", VERSION), 320, 200)
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .target_texture()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    let mut event_pump = sdl.event_pump()?;
    'running: loop {
        // get the inputs here
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => (),
            }
        }
        canvas.clear();
        canvas.present();
    }

    Ok(())
}
