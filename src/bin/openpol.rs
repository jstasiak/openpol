use flic::{FlicFile, RasterMut};
use openpol::image13h;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, TimerSubsystem};
use std::cmp;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path;

const VERSION: &'static str = env!("GIT_DESCRIPTION");

fn main() -> Result<(), String> {
    let game = Game::new();
    game.run()
}

struct Audio {
    data: Vec<u8>,
    position: usize,
}

impl AudioCallback for Audio {
    type Channel = u8;

    fn callback(&mut self, out: &mut [u8]) {
        let to_buffer = cmp::min(out.len(), self.data.len() - self.position);
        out[..to_buffer].copy_from_slice(&self.data[self.position..self.position + to_buffer]);
        self.position += to_buffer;
        // TODO repeat the audio just like we repeat the video. Going silent after the first play
        // for now.
        if self.position == self.data.len() {
            for x in out[to_buffer..].iter_mut() {
                *x = 0;
            }
        }
    }
}

struct Game {}

impl Game {
    pub fn new() -> Game {
        Game {}
    }

    pub fn run(&self) -> Result<(), String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let window = video
            .window(
                &format!("openpol {}", VERSION),
                image13h::SCREEN_WIDTH as u32 * 2,
                image13h::SCREEN_HEIGHT as u32 * 2,
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

        let texture_creator = canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                image13h::SCREEN_WIDTH as u32,
                image13h::SCREEN_HEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        let mut timer = sdl.timer()?;

        let mut audio_file =
            fs::File::open(&data_dir.join("I002.DAT")).map_err(|e| e.to_string())?;
        let mut audio_data = Vec::new();
        audio_file
            .read_to_end(&mut audio_data)
            .map_err(|e| e.to_string())?;
        let audio = sdl.audio()?;
        let desired_spec = AudioSpecDesired {
            freq: Some(22_050),
            channels: Some(1),
            samples: None,
        };

        let audio_device = audio.open_playback(None, &desired_spec, |_spec| Audio {
            data: audio_data,
            position: 0,
        })?;
        audio_device.resume();

        let mut flic = FlicFile::open(&data_dir.join("S002.DAT")).map_err(|e| e.to_string())?;
        assert_eq!(flic.width() as usize, image13h::SCREEN_WIDTH);
        assert_eq!(flic.height() as usize, image13h::SCREEN_HEIGHT);
        self.event_loop(
            &mut event_pump,
            &mut timer,
            &mut flic,
            &mut canvas,
            &mut texture,
        )
    }

    fn event_loop(
        &self,
        event_pump: &mut EventPump,
        timer: &mut TimerSubsystem,
        flic: &mut FlicFile,
        canvas: &mut WindowCanvas,
        texture: &mut Texture,
    ) -> Result<(), String> {
        let mut flic_buffer = vec![0; image13h::SCREEN_PIXELS];
        let mut flic_palette = vec![0; 3 * image13h::COLORS];

        let ms_per_frame = flic.speed_msec();
        let mut last_render = timer.ticks();
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
            let mut raster = RasterMut::new(
                image13h::SCREEN_WIDTH,
                image13h::SCREEN_HEIGHT,
                &mut flic_buffer,
                &mut flic_palette,
            );
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
}
