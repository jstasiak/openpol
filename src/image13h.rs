//! image13h images
//!
//! The original game uses [Mode 13h](https://en.wikipedia.org/wiki/Mode_13h) and contains a module
//! named image13h, so we'll mirror this naming here.
//!
//! # On-disk and in-memory format
//!
//! Image13h images are self-contained chunks of bytes of the following form:
//! `[ width ] [ height ] [ unknown ] [ data ]`
//!
//! * `width` and `height` are unsigned 2-byte little-endian integers
//! * `unknown` is a 2-byte chunk containing `1` and `0` (unsigned). Its purpose is currently
//!   unknown.
//! * `data` is `width * height` unsigned bytes containing color indices

use std::io;
use std::ops;

/// Mode 13h screen width.
pub const SCREEN_WIDTH: usize = 320;

/// Mode 13h screen height.
pub const SCREEN_HEIGHT: usize = 200;

/// Total number of pixels on the screen.
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

/// Mode 13h number of colors.
pub const COLORS: usize = 256;

/// The header size in bytes.
pub const HEADER_SIZE: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Image13h {
    data: Vec<u8>,
    width: usize,
    height: usize,
}

impl Image13h {
    /// Get the width of the image
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the height of the image.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get a reference to a slice containing the image data. The data is stored row by row.
    pub fn data(&self) -> &[u8] {
        &self.data[..]
    }

    /// Like `data`, but mutable.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[..]
    }

    /// Get the image contents of a particular line as a byte slice. `line` is 0-based. Since the
    /// data is stored line-by-line internally it's more optimal that way. If you want to access
    /// particular pixel just index into the slice.
    pub fn line(&self, line: usize) -> &[u8] {
        assert!(line < self.height);
        &self.data[line * self.width..(line + 1) * self.width]
    }

    /// Mutable variant of `line()`.
    pub fn mut_line(&mut self, line: usize) -> &mut [u8] {
        &mut self.data[line * self.width..(line + 1) * self.width]
    }

    /// Load an image from a reader. Extra content after the expected data is ignored.
    ///
    /// # Errors
    /// The method will return None if there's something wrong with the contents:
    /// * width or height equal 0
    /// * not enough bytes when reading
    pub fn load<T: io::Read>(mut reader: T) -> Option<Image13h> {
        let mut buffer = [0, 0];
        let width = match reader.read_exact(&mut buffer) {
            Err(_) => return None,
            Ok(_) => u16::from_le_bytes(buffer) as usize,
        };
        let height = match reader.read_exact(&mut buffer) {
            Err(_) => return None,
            Ok(_) => u16::from_le_bytes(buffer) as usize,
        };
        if width == 0 || height == 0 {
            return None;
        }
        match reader.read_exact(&mut buffer) {
            Err(_) => return None,
            Ok(_) => match buffer {
                [1, 0] => (),
                _ => return None,
            },
        }

        let mut data = vec![0; width * height];
        if reader.read_exact(&mut data).is_err() {
            return None;
        }
        Some(Image13h {
            width,
            height,
            data,
        })
    }

    /// Create an empty image with the specified dimensions. Empty means the image is filled with
    /// color 0.
    pub fn empty(width: usize, height: usize) -> Image13h {
        Image13h::filled_with_color(width, height, 0)
    }

    /// Create an image filled with desired color.
    pub fn filled_with_color(width: usize, height: usize, color: u8) -> Image13h {
        Image13h {
            width,
            height,
            data: vec![color; width * height],
        }
    }

    /// Save the image to a writer. Write errors will result in a panic.
    pub fn save<T: io::Write>(&self, mut writer: T) {
        for dim in &[self.width, self.height] {
            writer.write_all(&(*dim as u16).to_le_bytes()).unwrap();
        }
        writer.write_all(&[1, 0]).unwrap();
        writer.write_all(&self.data).unwrap();
    }

    /// Extract a `rect`-bound subimage from the image.
    pub fn subimage(&self, rect: &Rect) -> Image13h {
        let mut subimage = Self::empty(rect.width, rect.height);
        for (dst_line, src_line) in (rect.top..rect.beyond_bottom()).enumerate() {
            subimage
                .mut_line(dst_line)
                .copy_from_slice(&self.line(src_line)[rect.left..rect.beyond_right()]);
        }
        subimage
    }

    /// Blit another image into a rect in this image. The width and height of `image` and `rect`
    /// need to be the same.
    pub fn blit(&mut self, image: &Image13h, rect: &Rect) {
        for (src_line, dst_line) in (rect.top..rect.beyond_bottom()).enumerate() {
            self.mut_line(dst_line)[rect.left..rect.beyond_right()]
                .copy_from_slice(image.line(src_line));
        }
    }

    /// Fill the image with a color.
    pub fn fill(&mut self, color: u8) {
        let len = self.data.len();
        self.data.clear();
        self.data.resize(len, color);
    }
}

#[derive(Debug)]
pub struct Rect {
    /// The position of the left border, inclusive.
    pub left: usize,
    /// The position of the top border, inclusive.
    pub top: usize,
    /// The width of the rect.
    pub width: usize,
    /// The height of the rect.
    pub height: usize,
}

impl Rect {
    /// Construct a new `Rect` from coordinate ranges (exclusively-ended).
    pub fn from_ranges(x: ops::Range<usize>, y: ops::Range<usize>) -> Rect {
        Rect {
            left: x.start,
            top: y.start,
            width: x.end - x.start,
            height: y.end - y.start,
        }
    }

    /// Get the right-most x coordinate that's still in the rect.
    pub fn right_inclusive(&self) -> usize {
        self.beyond_right() - 1
    }

    /// Get the x coordinate that's one to the right of the right border of the rect.
    pub fn beyond_right(&self) -> usize {
        self.left + self.width
    }

    /// Get the bottom-most y coordinate that's still in the rect.
    pub fn bottom_inclusive(&self) -> usize {
        self.beyond_bottom() - 1
    }

    /// Get the y coordinate that's one below the bottom border of the rect.
    pub fn beyond_bottom(&self) -> usize {
        self.top + self.height
    }
}

pub fn indices_to_rgb<T: io::Write>(indices: &[u8], palette: &[u8], mut writer: T) {
    for color_index in indices {
        let palette_offset = *color_index as usize * 3;
        writer
            .write_all(&palette[palette_offset..palette_offset + 3])
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::image13h::{indices_to_rgb, Image13h, Rect};

    // 3 by 2 image, we have 1 byte extra at the end to see if we ignore it correctly
    static GOOD_DATA: [u8; 13] = [3, 0, 2, 0, 1, 0, 1, 2, 3, 4, 5, 6, 7];

    #[test]
    fn test_not_enough_data_is_an_error() {
        for size in 0..12 {
            dbg!(size);
            assert!(Image13h::load(&GOOD_DATA[0..size]).is_none());
        }
    }

    #[test]
    fn test_invalid_header_is_an_error() {
        // Zero width
        let bad_data1 = [0, 0, 1, 0, 1, 0];
        // Zero height
        let bad_data2 = [1, 0, 0, 0, 1, 0];
        // Bad unknown marker
        let bad_data3 = [1, 0, 1, 0, 0, 0, 0, 0];

        assert!(Image13h::load(&bad_data1[..]).is_none());
        assert!(Image13h::load(&bad_data2[..]).is_none());
        assert!(Image13h::load(&bad_data3[..]).is_none());
    }

    #[test]
    fn test_loading_works() {
        let image13h = Image13h::load(&GOOD_DATA[..]).unwrap();
        assert_eq!(image13h.width(), 3);
        assert_eq!(image13h.height(), 2);
        assert_eq!(image13h.line(0), [1, 2, 3]);
        assert_eq!(image13h.line(1), [4, 5, 6]);
    }

    #[test]
    fn test_saving_works() {
        let image13h = Image13h::load(&GOOD_DATA[..]).unwrap();
        let mut buffer = Vec::new();
        image13h.save(&mut buffer);
        assert_eq!(buffer, &GOOD_DATA[0..buffer.len()]);
    }

    #[test]
    fn test_rect_works() {
        let rect = Rect::from_ranges(0..10, 10..14);
        assert_eq!(rect.left, 0);
        assert_eq!(rect.top, 10);
        assert_eq!(rect.width, 10);
        assert_eq!(rect.height, 4);
        assert_eq!(rect.right_inclusive(), 9);
        assert_eq!(rect.beyond_right(), 10);
        assert_eq!(rect.bottom_inclusive(), 13);
        assert_eq!(rect.beyond_bottom(), 14);
    }

    #[test]
    fn test_subimage_works() {
        let image = Image13h::load(&GOOD_DATA[..]).unwrap();
        let subimage = image.subimage(&Rect::from_ranges(0..2, 0..2));
        let mut expected_subimage = Image13h::empty(2, 2);
        expected_subimage.mut_line(0).copy_from_slice(&[1, 2]);
        expected_subimage.mut_line(1).copy_from_slice(&[4, 5]);
        assert_eq!(subimage, expected_subimage);
    }

    #[test]
    fn test_fill_works() {
        let mut image = Image13h::empty(2, 1);
        let mut expected_image = Image13h::empty(2, 1);
        expected_image.mut_line(0).copy_from_slice(&[1, 1]);
        image.fill(1);
        assert_eq!(image, expected_image);
    }

    #[test]
    fn test_blit_works() {
        let mut main_image = Image13h::empty(3, 2);
        let mut subimage = Image13h::empty(2, 1);
        subimage.fill(1);
        main_image.blit(&subimage, &Rect::from_ranges(1..3, 1..2));
        let mut expected_image = Image13h::empty(3, 2);
        expected_image.mut_line(1)[1..3].copy_from_slice(&[1, 1]);
        assert_eq!(main_image, expected_image);
    }

    #[test]
    fn test_indices_to_rgb_works() {
        let indices = [1, 2, 0];
        let palette = [0, 1, 2, 10, 11, 12, 20, 21, 22];
        let expected_rgb = [10, 11, 12, 20, 21, 22, 0, 1, 2];
        let mut buffer = Vec::new();
        indices_to_rgb(&indices, &palette, &mut buffer);
        assert_eq!(buffer, expected_rgb);
    }
}
