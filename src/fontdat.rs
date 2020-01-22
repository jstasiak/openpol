//! font.dat data access operations.
//!
//! # font.dat file format
//!
//! font.dat is an [image13h-encoded](../image13h/index.html) image. It contains 3 rows of 13-pixel
//! high characters. The rows start at the following pixels (all indices/offsets/positions are 0-based
//! unless specified otherwise) and have the following number of characters:
//! * The first row: line 8, column 7, 33 characters
//! * The second row: line 32, column 8, 31 characters
//! * The third row: line 56, column 7, 27 characters
//!
//! The widths of the characters are hardcoded, `CHARACTER_WIDTHS` array is provided for convenience.

use crate::image13h;
use std::io;

/// Where do rows start.
pub const ROWS: usize = 3;

/// How many characters are there, in total.
pub const CHARACTERS: usize = 91;

pub const CHARACTER_HEIGHT: usize = 13;

/// Where do rows start, (x, y).
pub const ROW_OFFSETS: [(usize, usize); ROWS] = [(7, 8), (8, 32), (7, 56)];

/// How many characters are there in every row.
pub const ROW_CHARACTERS: [usize; ROWS] = [33, 31, 27];

/// The width and height of the smallest image that's capable of storing a font.
pub const MINIMUM_IMAGE_DIMENSIONS: (usize, usize) = (223, 69);

/// The widths of all characters.
pub const CHARACTER_WIDTHS: [usize; CHARACTERS] = [
    // The first row
    4, 2, 4, 6, 6, 6, 6, 6, 4, 4, 6, 6, 2, 4, 2, 6, 6, 5, 6, 6, 6, 6, 6, 6, 6, 6, 2, 2, 8, 6, 8, 6,
    7, // The second row
    8, 7, 7, 7, 6, 6, 8, 7, 2, 5, 7, 6, 8, 7, 8, 6, 8, 7, 7, 6, 7, 8, 11, 7, 8, 7, 7, 7, 7, 6, 7,
    // The third row
    4, 6, 6, 6, 6, 6, 4, 6, 6, 2, 2, 5, 2, 8, 6, 6, 6, 6, 4, 6, 3, 6, 6, 10, 6, 6, 6,
];

/// The x positions of the characters in the font image.
pub const CHARACTER_X_POSITIONS: [usize; CHARACTERS] = [
    // THe first row
    7, 11, 13, 17, 23, 29, 35, 41, 47, 51, 55, 61, 67, 69, 73, 75, 81, 87, 92, 98, 104, 110, 116,
    122, 128, 134, 140, 142, 144, 152, 158, 166, 172, // The second row
    8, 16, 23, 30, 37, 43, 49, 57, 64, 66, 71, 78, 84, 92, 99, 107, 113, 121, 128, 135, 141, 148,
    156, 167, 174, 182, 189, 196, 203, 210, 216, // The third row
    7, 11, 17, 23, 29, 35, 41, 45, 51, 57, 59, 61, 66, 68, 76, 82, 88, 94, 100, 104, 110, 113, 119,
    125, 135, 141, 147,
];

#[derive(Debug, Eq, PartialEq)]
pub struct Fontdat {
    glyphs: Vec<image13h::Image13h>,
}

impl Fontdat {
    /// Load a font from a reader. This function will return None if:
    ///
    /// * The image can't be loaded
    /// * The image loaded is too small (see `MINIMUM_IMAGE_DIMENSIONS`)
    pub fn load<T: io::Read>(reader: T) -> Option<Fontdat> {
        let image = match image13h::Image13h::load(reader) {
            None => return None,
            Some(image) => image,
        };
        if (image.width(), image.height()) < MINIMUM_IMAGE_DIMENSIONS {
            return None;
        }
        let mut glyphs = Vec::new();
        for character in 0..CHARACTERS {
            let rect = character_rect(character);
            let glyph = image.subimage(&rect);
            glyphs.push(glyph);
        }
        Some(Fontdat { glyphs })
    }

    /// Create a new empty font (all characters are filled with color 0).
    pub fn empty() -> Fontdat {
        let mut glyphs = Vec::new();
        for character in 0..CHARACTERS {
            let rect = character_rect(character);
            let glyph = image13h::Image13h::empty(rect.width, rect.height);
            glyphs.push(glyph);
        }
        Fontdat { glyphs }
    }

    /// Save the font to a writer.
    pub fn save<T: io::Write>(&self, writer: T) {
        let mut image =
            image13h::Image13h::empty(MINIMUM_IMAGE_DIMENSIONS.0, MINIMUM_IMAGE_DIMENSIONS.1);
        for character in 0..CHARACTERS {
            let rect = character_rect(character);
            image.blit(&self.glyphs[character], rect.left, rect.top);
        }
        image.save(writer);
    }

    /// Get a reference to a character glyph.
    pub fn glyph(&self, character: usize) -> &image13h::Image13h {
        &self.glyphs[character]
    }

    /// Get a mutable reference to character glyph.
    pub fn glyph_mut(&mut self, character: usize) -> &mut image13h::Image13h {
        &mut self.glyphs[character]
    }
}

pub fn character_rect(character: usize) -> image13h::Rect {
    debug_assert!(character < CHARACTERS);
    let line = if character < ROW_CHARACTERS[0] {
        0
    } else if character < ROW_CHARACTERS[0] + ROW_CHARACTERS[1] {
        1
    } else {
        2
    };

    let x = CHARACTER_X_POSITIONS[character];
    let y = ROW_OFFSETS[line].1;
    image13h::Rect::from_ranges(x..x + CHARACTER_WIDTHS[character], y..y + CHARACTER_HEIGHT)
}

#[cfg(test)]
mod tests {
    use crate::fontdat::{Fontdat, CHARACTERS};
    use std::fs;

    #[test]
    fn test_loading_and_saving_works() {
        // dummy_font.dat contains a font with every character filled with color equal to the
        // character's index + 100.
        let dummy_font_dat = fs::read("dummy_font.dat").unwrap();
        let fontdat = Fontdat::load(&dummy_font_dat[..]).unwrap();
        let mut expected_fontdat = Fontdat::empty();
        for i in 0..CHARACTERS {
            expected_fontdat.glyph_mut(i).fill(100 + i as u8);
        }
        // First let's verify that after loading from disk we get the expected glyphs...
        assert_eq!(fontdat, expected_fontdat);
        // ...then make sure that after saving a font we get the exact save binary content.
        let mut buf = Vec::new();
        fontdat.save(&mut buf);
        assert_eq!(buf, dummy_font_dat);
    }
}
