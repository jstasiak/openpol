use openpol::sounddat;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} FILE [SOUND]

When no SOUND is passed – list all sounds in the FILE.
SOUND is a 0-based number of a sound in the FILE. If pressent – dump the sound data to stdout.

        ",
        program,
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (path, sound) = if args.len() == 2 {
        (&args[1], None)
    } else if args.len() == 3 {
        (
            &args[1],
            match usize::from_str_radix(&args[2], 10) {
                Ok(value) => Some(value),
                Err(_) => usage(&args[0]),
            },
        )
    } else {
        usage(&args[0]);
    };
    let mut file = fs::File::open(path).unwrap();
    let sounddat = sounddat::Sounddat::load(&mut file).unwrap();
    match sound {
        Some(sound) => io::stdout().write_all(sounddat.sound_data(sound)).unwrap(),
        None => {
            println!("Sounds in {}:", path);
            for i in 0..sounddat.sounds() {
                println!("{}: {} bytes", i, sounddat.sound_data(i).len());
            }
        }
    }
}
