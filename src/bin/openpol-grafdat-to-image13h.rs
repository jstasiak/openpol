use openpol::grafdat;
use openpol::image13h;
use std::env;
use std::fs;
use std::io;
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {program} GRAFDAT_FILE

Combine images found in a graf.dat file into a single image13h image. The result is printed to stdout",
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
    let mut image_all = image13h::Image13h::empty(
        grafdat::IMAGE_DIMENSIONS.0,
        grafdat::IMAGE_DIMENSIONS.1 * grafdat::IMAGES,
    );

    let images = grafdat.to_images();
    for (i, image) in images.iter().enumerate() {
        let yoffset = i * grafdat::IMAGE_DIMENSIONS.1;
        image_all.blit(image, 0, yoffset);
    }
    image_all.save(io::stdout());
}
