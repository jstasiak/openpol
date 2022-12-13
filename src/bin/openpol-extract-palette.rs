use openpol::paldat;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {program} FILE [PALETTE]

When no PALETTE is passed – print the number of palettes in FILE.
PALETTE is a 0-based index of a palette in the FILE. If pressent – dump the palette data to stdout.

        ",
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (path, palette) = if args.len() == 2 {
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
    let paldat = paldat::Paldat::load(&mut file).unwrap();
    match palette {
        Some(palette) => io::stdout()
            .write_all(paldat.palette_data(palette))
            .unwrap(),
        None => {
            println!("The number of palettes in {}: {}", path, paldat.palettes());
        }
    }
}
