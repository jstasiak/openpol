use openpol::grafdat;
use openpol::image13h;
use std::env;
use std::fs;
use std::io;
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} GRAFDAT_FILE

Combine images found in a graf.dat file into a single image13h image. The result is printed to stdout",
        program,
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        usage(&args[0]);
    }

    let grafdat_file = fs::File::open(&args[1]).unwrap();
    let grafdat = grafdat::Grafdat::load(grafdat_file).unwrap();
    let mut image = image13h::Image13h::empty(
        grafdat::IMAGE_DIMENSIONS.0,
        grafdat::IMAGE_DIMENSIONS.1 * grafdat::IMAGES,
    );

    for i in 0..grafdat::IMAGES {
        let yoffset = i * grafdat::IMAGE_DIMENSIONS.1;
        image.blit(
            grafdat.image(i),
            &image13h::Rect::from_ranges(
                0..grafdat::IMAGE_DIMENSIONS.0,
                yoffset..yoffset + grafdat::IMAGE_DIMENSIONS.1,
            ),
        );
    }
    image.save(io::stdout());
}
