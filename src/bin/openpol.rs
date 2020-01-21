use flic::{FlicFile, RasterMut};
use openpol::{grafdat, image13h, paldat};
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, TimerSubsystem};
use std::cmp;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path;

const VERSION: &str = env!("GIT_DESCRIPTION");

fn main() -> Result<(), String> {
    let mut game = Game::new()?;
    game.run()
}

struct Audio {
    data: Vec<u8>,
    position: usize,
    silence: u8,
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
                *x = self.silence;
            }
        }
    }
}

struct Game {
    data_dir: path::PathBuf,
    grafdat: grafdat::Grafdat,
    paldat: paldat::Paldat,
    screen: image13h::Image13h,
    palette: Vec<u8>,
}

impl Game {
    pub fn new() -> Result<Game, String> {
        let args: Vec<String> = env::args().skip(1).collect();
        if args.len() != 1 {
            return Err("Usage: openpol GAMEDIR".to_string());
        }
        let root_dir = path::Path::new(&args[0]);
        let data_dir = root_dir.join("data");

        Ok(Game {
            data_dir: data_dir.to_path_buf(),
            paldat: paldat::Paldat::load(fs::File::open(root_dir.join("pal.dat")).unwrap())
                .unwrap(),
            grafdat: grafdat::Grafdat::load(fs::File::open(root_dir.join("graf.dat")).unwrap())
                .unwrap(),
            screen: image13h::Image13h::empty_screen_sized(),
            palette: vec![0; paldat::PALETTE_SIZE_IN_BYTES],
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
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

        let texture_creator = canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                image13h::SCREEN_WIDTH as u32,
                image13h::SCREEN_HEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        let mut timer = sdl.timer()?;
        let audio = sdl.audio()?;
        let desired_spec = AudioSpecDesired {
            freq: Some(22_050),
            channels: Some(1),
            samples: None,
        };

        let mut audio_device = audio.open_playback(None, &desired_spec, |spec| Audio {
            data: Vec::new(),
            position: 0,
            silence: spec.silence,
        })?;
        audio_device.resume();

        self.event_loop(
            &mut event_pump,
            &mut timer,
            &mut canvas,
            &mut texture,
            &mut audio_device,
        )
    }

    fn event_loop(
        &mut self,
        event_pump: &mut EventPump,
        timer: &mut TimerSubsystem,
        canvas: &mut WindowCanvas,
        texture: &mut Texture,
        audio_device: &mut AudioDevice<Audio>,
    ) -> Result<(), String> {
        let mut intro = Intro::new(&self.data_dir)?;

        let mut last_render = timer.ticks();
        while intro.running() {
            // get the inputs here
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => intro.stop(audio_device),
                    Event::KeyDown { .. } => intro.next(audio_device),
                    _ => (),
                }
            }
            let now = timer.ticks();
            let dt = now - last_render;
            last_render = now;
            intro.update(dt, audio_device);

            // NOTE: pitch is assumed to be equal to video width * 3 bytes (RGB), eg. there are no
            // holes between rows in the buffer.
            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                intro.display(buffer)
            })?;
            canvas.clear();
            canvas.copy(&texture, None, None)?;
            canvas.present();
        }

        let mut game_running = true;
        self.palette[..].copy_from_slice(self.paldat.palette_data(2));
        self.screen.blit_whole(self.grafdat.main_menu(), 0, 0);
        while game_running {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => game_running = false,
                    Event::KeyDown { .. } => game_running = false,
                    _ => (),
                }
            }
            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                image13h::indices_to_rgb(self.screen.data(), &self.palette, buffer);
            })?;
            canvas.clear();
            canvas.copy(&texture, None, None)?;
            canvas.present();
        }
        Ok(())
    }
}

struct Intro<'a> {
    flic: Option<FlicFile>,
    since_last_render: u32,
    flic_buffer: Vec<u8>,
    flic_palette: Vec<u8>,
    data_dir: &'a path::Path,
    buffer_changed: bool,
    current_intro: u8,
    running: bool,
}

impl<'a> Intro<'a> {
    pub fn new(data_dir: &'a path::Path) -> Result<Intro<'a>, String> {
        Ok(Intro {
            flic: None,
            since_last_render: 0,
            flic_buffer: vec![0; image13h::SCREEN_PIXELS],
            flic_palette: vec![0; 3 * image13h::COLORS],
            data_dir,
            buffer_changed: false,
            current_intro: 0,
            running: true,
        })
    }

    pub fn update(&mut self, ticks: u32, audio_device: &mut AudioDevice<Audio>) {
        let flic = match &mut self.flic {
            None => match self.current_intro {
                i @ 0..=2 => {
                    let flic =
                        FlicFile::open(&self.data_dir.join(format!("S00{}.DAT", i))).unwrap();
                    assert_eq!(flic.width() as usize, image13h::SCREEN_WIDTH);
                    assert_eq!(flic.height() as usize, image13h::SCREEN_HEIGHT);
                    self.flic = Some(flic);

                    match fs::File::open(&self.data_dir.join(format!("I00{}.DAT", i))) {
                        Err(_) => (),
                        Ok(mut audio_file) => {
                            // The IXXX.DAT files have a 4-byte little-endian integer header that
                            // contains the audio data size (the size of the whole file should be
                            // audio data size + 4 bytes for the header).
                            let mut len_buf = [0; 4];
                            audio_file.read_exact(&mut len_buf).unwrap();
                            let expected_len = u32::from_le_bytes(len_buf) as usize;

                            let mut audio_data = Vec::new();
                            audio_file.read_to_end(&mut audio_data).unwrap();
                            assert_eq!(audio_data.len(), expected_len);

                            let mut audio_lock = audio_device.lock();
                            audio_lock.data = audio_data;
                            audio_lock.position = 0;
                        }
                    };

                    self.flic.as_mut().unwrap()
                }
                _ => return,
            },
            Some(flic) => flic,
        };

        let ms_per_frame = flic.speed_msec();

        self.since_last_render += ticks;
        let buffer_changed = self.since_last_render >= ms_per_frame;
        if buffer_changed {
            self.buffer_changed = true;
            let mut raster = RasterMut::new(
                image13h::SCREEN_WIDTH,
                image13h::SCREEN_HEIGHT,
                &mut self.flic_buffer,
                &mut self.flic_palette,
            );
            while self.since_last_render >= ms_per_frame {
                let playback_result = flic.read_next_frame(&mut raster).unwrap();
                if playback_result.ended {
                    self.next(audio_device);
                    return;
                } else {
                    self.since_last_render -= ms_per_frame;
                }
            }
        }
    }

    pub fn display(&mut self, buffer: &mut [u8]) {
        if self.buffer_changed {
            image13h::indices_to_rgb(&self.flic_buffer, &self.flic_palette, buffer);
            self.buffer_changed = false;
        }
    }

    pub fn running(&self) -> bool {
        self.running && self.current_intro < 3
    }

    pub fn stop(&mut self, audio_device: &mut AudioDevice<Audio>) {
        self.running = false;
        clear_audio(audio_device);
    }

    pub fn next(&mut self, audio_device: &mut AudioDevice<Audio>) {
        self.since_last_render = 0;
        self.flic = None;
        self.current_intro += 1;
        clear_audio(audio_device);
    }
}

fn clear_audio(audio_device: &mut AudioDevice<Audio>) {
    let mut audio_lock = audio_device.lock();
    audio_lock.data = Vec::new();
    audio_lock.position = 0;
}
