//! pal.dat data access operations.
//!
//! # pal.dat file format
//! * The file consists of concatenated 768-byte color palettes.
//! * Each palette contains 256 3-byte colors.
//! * Each 3-byte color definition consists of red, green and blue component values, 1 unsigned byte
//!   each.
//!
//! The original game uses [Mode 13h](https://en.wikipedia.org/wiki/Mode_13h). As mode 13h only
//! supports 6 bits per channel the full byte values cannot be used. The original game shifts the
//! values by two bits to the right (which is effectively divinding by four), therefore removing
//! the two least significant bits and leaving the six most significant ones. This module doesn't
//! truncate the values, therefore full 24-bit colors are used, as long as present in `pal.dat`.
//!
//! # Example
//!
//! An `openpol-extract-palette` sample binary which uses this code is provided. You can display
//! a palette (palette number 3 /0-based/ in this case) like this (the code depends on ImageMagick
//! being present in the system, the palette is displayed as 16x16 pixel square):
//!
//! `convert -depth 8 -size 16x16 rgb:<(cargo run --bin openpol-extract-palette PAL.DAT 3) image.png`
//!
//! Now view `image.png` with the image viewer of your choice.
use std::io;

/// A way to access pal.dat contents.
pub struct Paldat {
    data: Vec<u8>,
}

pub const PALETTE_SIZE_IN_BYTES: usize = 768;

impl Paldat {
    /// Load pal.dat contents. All of it is read into memory.
    ///
    /// # Errors
    /// The code will panic if `reader` cannot read to end. If the number of bytes is not a
    /// multiple of 768 bytes (invalid file) the function will return `None`.
    pub fn load<T: io::Read>(mut reader: T) -> Option<Paldat> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();
        if data.len() % PALETTE_SIZE_IN_BYTES != 0 {
            None
        } else {
            Some(Paldat { data })
        }
    }

    /// The number of palettes in the file.
    pub fn palettes(&self) -> usize {
        self.data.len() / PALETTE_SIZE_IN_BYTES
    }

    /// The `palette`'s data (`palette` is 0-based). The data is to be interpreted as described by the
    /// [module's documentation on the palette format](index.html).
    pub fn palette_data(&self, palette: usize) -> &[u8] {
        &self.data[palette * PALETTE_SIZE_IN_BYTES..(palette + 1) * PALETTE_SIZE_IN_BYTES]
    }
}

#[cfg(test)]
mod tests {
    use crate::paldat::Paldat;

    #[test]
    fn test_paldat_loading_works() {
        let data: Vec<u8> = (0..(768 as u16 * 2)).map(|v| (v >> 3) as u8).collect();
        let paldat = Paldat::load(&data[..]).unwrap();
        assert_eq!(paldat.palettes(), 2);
        assert_eq!(paldat.palette_data(0), &data[0..768]);
        assert_eq!(paldat.palette_data(1), &data[768..768 * 2]);
    }
}
