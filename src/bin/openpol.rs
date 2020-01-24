use flic::{FlicFile, RasterMut};
use openpol::{audio, grafdat, image13h, paldat};
use sdl2::audio::{AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, TimerSubsystem};
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path;

const VERSION: &str = env!("GIT_DESCRIPTION");

fn main() -> Result<(), String> {
    let game = Game::new()?;
    game.run()
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

    pub fn run(self) -> Result<(), String> {
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

        let mut audio_device = audio.open_playback(None, &desired_spec, |spec| audio::Audio {
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
        mut self,
        event_pump: &mut EventPump,
        timer: &mut TimerSubsystem,
        canvas: &mut WindowCanvas,
        texture: &mut Texture,
        audio_device: &mut AudioDevice<audio::Audio>,
    ) -> Result<(), String> {
        let mut last_render = timer.ticks();
        let mut behavior: Box<dyn Behavior> = Box::new(Intro::new(self.data_dir.clone()).unwrap());
        let mut running = true;
        while running {
            let mut button_pressed = false;
            // get the inputs here
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => {
                        running = false;
                    }
                    Event::KeyDown { .. } => {
                        button_pressed = true;
                    }
                    _ => (),
                }
            }
            let now = timer.ticks();
            let dt = now - last_render;
            last_render = now;
            // NOTE: pitch is assumed to be equal to video width * 3 bytes (RGB), eg. there are no
            // holes between rows in the buffer.
            texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                if let Some(new_behavior) =
                    behavior.update(&mut self, button_pressed, dt, audio_device, buffer)
                {
                    behavior = new_behavior;
                }
            })?;
            canvas.clear();
            canvas.copy(&texture, None, None)?;
            canvas.present();
        }
        Ok(())
    }
}

trait Behavior {
    fn update(
        &mut self,
        game: &mut Game,
        button_pressed: bool,
        ticks: u32,
        audio_device: &mut AudioDevice<audio::Audio>,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>>;
}

struct Intro {
    flic: Option<FlicFile>,
    since_last_render: u32,
    flic_buffer: Vec<u8>,
    flic_palette: Vec<u8>,
    data_dir: path::PathBuf,
    current_intro: u8,
}

impl Intro {
    pub fn new(data_dir: path::PathBuf) -> Result<Intro, String> {
        Ok(Intro {
            flic: None,
            since_last_render: 0,
            flic_buffer: vec![0; image13h::SCREEN_PIXELS],
            flic_palette: vec![0; 3 * image13h::COLORS],
            data_dir,
            current_intro: 0,
        })
    }

    pub fn next(&mut self, audio_device: &mut AudioDevice<audio::Audio>) {
        self.since_last_render = 0;
        self.flic = None;
        self.current_intro += 1;
        audio::clear_audio(audio_device);
    }
}

impl Behavior for Intro {
    fn update(
        &mut self,
        _game: &mut Game,
        button_pressed: bool,
        ticks: u32,
        audio_device: &mut AudioDevice<audio::Audio>,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>> {
        if button_pressed {
            self.next(audio_device);
        }

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
                _ => return Some(Box::new(MainMenu {})),
            },
            Some(flic) => flic,
        };

        let ms_per_frame = flic.speed_msec();

        self.since_last_render += ticks;
        let buffer_changed = self.since_last_render >= ms_per_frame;
        if buffer_changed {
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
                    return Some(Box::new(MainMenu {}));
                } else {
                    self.since_last_render -= ms_per_frame;
                }
            }
            image13h::indices_to_rgb(&self.flic_buffer, &self.flic_palette, buffer);
        }
        None
    }
}

struct MainMenu {}

impl Behavior for MainMenu {
    fn update(
        &mut self,
        game: &mut Game,
        _button_pressed: bool,
        _ticks: u32,
        _audio_device: &mut AudioDevice<audio::Audio>,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>> {
        // TODO stop copying every frame
        game.palette[..].copy_from_slice(game.paldat.palette_data(2));
        game.screen.blit(game.grafdat.main_menu(), 0, 0);
        // TODO Stop converting and copying data every frame unnecessarily
        image13h::indices_to_rgb(game.screen.data(), &game.palette, buffer);
        None
    }
}
