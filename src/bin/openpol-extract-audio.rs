use openpol::sounddat;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage:
{} FILE - prints number of sounds in FILE
{} --extract SOUND FILE - prints the bytes of sound SOUND (0-based) from FILE",
        program, program
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let sound;
    let path = if args.len() == 2 {
        sound = None;
        &args[1]
    } else if args.len() == 4 {
        if args[1] != "--extract" {
            usage(&args[0]);
        }
        sound = Some(match usize::from_str_radix(&args[2], 10) {
            Ok(value) => value,
            Err(_) => usage(&args[0]),
        });
        &args[3]
    } else {
        usage(&args[0]);
    };
    let mut file = fs::File::open(path).unwrap();
    let sounddat = sounddat::Sounddat::load(&mut file).unwrap();
    match sound {
        Some(sound) => io::stdout().write_all(sounddat.sound_data(sound)).unwrap(),
        None => println!("Number of sounds in {}: {}", path, sounddat.sounds()),
    }
}
