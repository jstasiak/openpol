use openpol::image13h;
use openpol::paldat;
use openpol::ppm;
use std::env;
use std::fs;
use std::io;
use std::process;

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} IMAGE13H_FILE PALETTE_FILE PALETTE_INDEX

Convert an image13h image from IMAGE13H_FILE using PALETTE_INDEX from PALETTE_FILE to an RGB image
using PPM text format. The PPM image is printed to stdout.",
        program,
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        usage(&args[0]);
    }

    let image_file = fs::File::open(&args[1]).unwrap();
    let image13h = image13h::Image13h::load(image_file).unwrap();

    let palette_file = fs::File::open(&args[2]).unwrap();
    let paldat = paldat::Paldat::load(palette_file).unwrap();

    let palette_index = usize::from_str_radix(&args[3], 10).unwrap();
    let palette = paldat.palette_data(palette_index);

    let mut rgb = Vec::new();
    image13h::indices_to_rgb(image13h.data(), palette, &mut rgb);
    ppm::write_ppm(image13h.width(), image13h.height(), &rgb[..], io::stdout()).unwrap();
}
