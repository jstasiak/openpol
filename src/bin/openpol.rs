use flic::{FlicFile, RasterMut};
use openpol::{grafdat, image13h, paldat, sounddat};
use sdl2::event::Event;
use sdl2::get_error;
use sdl2::mixer;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::{EventPump, TimerSubsystem};

use std::cmp;
use std::convert::TryInto;
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
    root_dir: path::PathBuf,
    data_dir: path::PathBuf,
    grafdat: grafdat::Grafdat,
    paldat: paldat::Paldat,
    music: Option<mixer::Music<'static>>,
    sounds: Vec<mixer::Chunk>,
}

impl Game {
    /// Play music track. Music files are expected to be named trackX.ogg and be placed in the
    /// music subdirectory of root game directory (X is the track number). In order to gracefully
    /// handle missing music files we simply print an error message to stderr when we can't play
    /// a track. Note that music previously played (if any) is stopped regardless of the success.
    ///
    /// Track numbers are 2-based (technically 1-based, but the data is the first track on the
    /// disc, so...) to keep the same numbering scheme as the original game.
    pub fn play_music_maybe(&mut self, track: usize) {
        let file_path = self
            .root_dir
            .join("music")
            .join(format!("track{}.ogg", track));
        if file_path.is_file() {
            match mixer::Music::from_file(&file_path) {
                Ok(music) => {
                    music.play(-1).unwrap();
                    self.music = Some(music);
                }
                Err(e) => {
                    self.music = None;
                    eprintln!("Cannot load music from {:?}: {}", file_path, e);
                }
            }
        } else {
            self.music = None;
            eprintln!("Music file {:?} not found", file_path);
        }
    }
}

impl Game {
    pub fn new() -> Result<Game, String> {
        let args: Vec<String> = env::args().skip(1).collect();
        if args.len() != 1 {
            return Err("Usage: openpol GAMEDIR".to_string());
        }
        let root_dir = path::Path::new(&args[0]);
        let data_dir = root_dir.join("data");

        // This needs to happen before we try to load any music or sound chunks
        mixer::open_audio(22_050, mixer::AUDIO_U8, 1, 1_024)?;
        mixer::init(mixer::InitFlag::OGG)?;
        // 16 is a semi-random number here
        mixer::allocate_channels(16);

        Ok(Game {
            root_dir: root_dir.to_path_buf(),
            data_dir,
            music: None,
            paldat: paldat::Paldat::load(fs::File::open(root_dir.join("pal.dat")).unwrap())
                .unwrap(),
            grafdat: grafdat::Grafdat::load(fs::File::open(root_dir.join("graf.dat")).unwrap())
                .unwrap(),
            sounds: sounddat::Sounddat::load(
                fs::File::open(root_dir.join("data").join("sound.dat")).unwrap(),
            )
            .unwrap()
            .into_vecs()
            .into_iter()
            .map(|v| buffer_into_chunk(v.into_boxed_slice()).unwrap())
            .collect(),
        })
    }

    pub fn run(self) -> Result<(), String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        // This show_cursor() call needs to happen *after* the video subsystem is initialized,
        // otherwise it'll silently do nothing.
        sdl.mouse().show_cursor(false);
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

        self.event_loop(&mut event_pump, &mut timer, &mut canvas, &mut texture)
    }

    fn event_loop(
        mut self,
        event_pump: &mut EventPump,
        timer: &mut TimerSubsystem,
        canvas: &mut WindowCanvas,
        texture: &mut Texture,
    ) -> Result<(), String> {
        let mut last_render = timer.ticks();
        let mut behavior: Box<dyn Behavior> = Box::new(Intro::new(self.data_dir.clone()).unwrap());
        let mut running = true;
        let mut input = Input {
            mouse_position: (0, 0),
        };
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
                    Event::MouseMotion { x, y, .. } => {
                        // We currently have to divide the coordinates by two, because we
                        // scale the screen to be double the game's original resolution.
                        input.mouse_position = (x as usize / 2, y as usize / 2);
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
                    behavior.update(&mut self, button_pressed, dt, &input, buffer)
                {
                    behavior = new_behavior;
                }
            })?;
            canvas.clear();
            canvas.copy(texture, None, None)?;
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
        input: &Input,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>>;
}

struct Input {
    pub mouse_position: (usize, usize),
}

struct Intro {
    flic: Option<FlicFile>,
    chunk: Option<mixer::Chunk>,
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
            chunk: None,
            since_last_render: 0,
            flic_buffer: vec![0; image13h::SCREEN_PIXELS],
            flic_palette: vec![0; 3 * image13h::COLORS],
            data_dir,
            current_intro: 0,
        })
    }

    pub fn next(&mut self) {
        self.since_last_render = 0;
        self.flic = None;
        self.chunk = None;
        self.current_intro += 1;
    }
}

impl Behavior for Intro {
    fn update(
        &mut self,
        _game: &mut Game,
        button_pressed: bool,
        ticks: u32,
        _input: &Input,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>> {
        if button_pressed {
            self.next();
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

                            let chunk = buffer_into_chunk(audio_data.into_boxed_slice()).unwrap();
                            mixer::Channel::all().play(&chunk, 0).unwrap();
                            self.chunk = Some(chunk);
                        }
                    };

                    self.flic.as_mut().unwrap()
                }
                _ => return Some(Box::new(MainMenu::new())),
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
                    self.next();
                    return None;
                } else {
                    self.since_last_render -= ms_per_frame;
                }
            }
            image13h::indices_to_rgb(&self.flic_buffer, &self.flic_palette, buffer);
        }
        None
    }
}

struct MainMenu {
    music_playing: bool,
}

impl MainMenu {
    pub fn new() -> MainMenu {
        MainMenu {
            music_playing: false,
        }
    }
}

impl Behavior for MainMenu {
    fn update(
        &mut self,
        game: &mut Game,
        button_pressed: bool,
        _ticks: u32,
        input: &Input,
        buffer: &mut [u8],
    ) -> Option<Box<dyn Behavior>> {
        if !self.music_playing {
            game.play_music_maybe(2);
            self.music_playing = true;
        }
        // TODO stop copying every frame
        let mut screen = image13h::Image13h::empty_screen_sized();
        screen.blit(game.grafdat.main_menu(), 0, 0);
        // Yes, the main menu cursor image comes from the buttons image array.
        let cursor = game.grafdat.button(6);
        screen.blit_with_transparency(
            cursor,
            // TODO Implement blitting that handles the source image crossing the destination image
            // boundary gracefully. We need this to display the cursor correctly near the right and
            // bottom borders. Clipping the blitting coordinates for now but it's a hack.
            cmp::min(
                image13h::SCREEN_WIDTH - cursor.width(),
                input.mouse_position.0,
            ),
            cmp::min(
                image13h::SCREEN_HEIGHT - cursor.height(),
                input.mouse_position.1,
            ),
        );

        if button_pressed {
            mixer::Channel::all().play(&game.sounds[0], 0).unwrap();
        }
        // TODO Stop converting and copying data every frame unnecessarily
        image13h::indices_to_rgb(screen.data(), game.paldat.palette_data(2), buffer);
        None
    }
}

fn buffer_into_chunk(buffer: Box<[u8]>) -> Result<mixer::Chunk, String> {
    let len = buffer.len();
    let mut raw = unsafe {
        sdl2_sys::mixer::Mix_QuickLoad_RAW(
            Box::into_raw(buffer) as *mut u8,
            len.try_into().unwrap(),
        )
    };
    if raw.is_null() {
        Err(get_error())
    } else {
        // allocated set to 1 makes SDL believe it allocated the memory for the chunk, so, when we drop
        // the Chunk, SDL_FreeChunk will be called and it'll deallocate the memory. I believe this is
        // fine, as long as free() is enough to deallocate Box<[u8]> (no special routines to call) and
        // SDL uses the same allocator as Rust does (few tests confirm that).
        unsafe {
            (*raw).allocated = 1;
        }
        Ok(mixer::Chunk { raw, owned: true })
    }
}
