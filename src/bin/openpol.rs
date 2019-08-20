use flic::{FlicFile, RasterMut};
use openpol::image13h;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use std::env;
use std::path;

const VERSION: &'static str = env!("GIT_DESCRIPTION");

fn main() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window(
            &format!("openpol {}", VERSION),
            image13h::SCREEN_WIDTH as u32,
            image13h::SCREEN_HEIGHT as u32,
        )
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

    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: openpol GAMEDIR".to_string());
    }
    let root_dir = path::Path::new(&args[0]);
    let data_dir = root_dir.join("data");
    let mut flic = FlicFile::open(&data_dir.join("S002.DAT")).map_err(|e| e.to_string())?;
    let flic_width = flic.width() as usize;
    let flic_height = flic.height() as usize;
    let mut flic_buffer = vec![0; flic_width * flic_height];
    let mut flic_palette = vec![0; 3 * image13h::COLORS];

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            flic_width as u32,
            flic_height as u32,
        )
        .map_err(|e| e.to_string())?;

    let mut timer = sdl.timer()?;
    let mut last_render = timer.ticks();
    let ms_per_frame = flic.speed_msec();

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

        let now = timer.ticks();
        let buffer_changed = now > last_render + ms_per_frame;
        let mut raster =
            RasterMut::new(flic_width, flic_height, &mut flic_buffer, &mut flic_palette);
        while last_render < now - ms_per_frame {
            flic.read_next_frame(&mut raster)
                .map_err(|e| e.to_string())?;
            last_render += ms_per_frame;
        }
        if buffer_changed {
            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                // NOTE: pitch is assumed to be equal to video width * 3 bytes (RGB), eg. there are no
                // holes between rows in the buffer.
                image13h::indices_to_rgb(&flic_buffer, &flic_palette, buffer)
            })?;
        }

        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();
    }

    Ok(())
}
