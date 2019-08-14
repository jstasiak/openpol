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

    /// Get the image contents of a particular line as a byte slice. `line` is 0-based. Since the
    /// data is stored line-by-line internally it's more optimal that way. If you want to access
    /// particular pixel just index into the slice.
    pub fn line(&self, line: usize) -> &[u8] {
        assert!(line < self.height);
        &self.data[line * self.width..(line + 1) * self.width]
    }

    /// Load an image from a reader. Extra content after the expected data is ignored.
    ///
    /// # Errors
    /// The method will return None if there's something wrong with the contents:
    /// * width or height equal 0
    /// * not enough bytes when reading
    pub fn load<T: io::Read>(reader: &mut T) -> Option<Image13h> {
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
        match reader.read_exact(&mut data) {
            Err(_) => return None,
            Ok(_) => (),
        };
        Some(Image13h {
            width,
            height,
            data,
        })
    }

    /// Save the image to a writer. Write errors will result in a panic.
    pub fn save<T: io::Write>(&self, writer: &mut T) {
        for dim in &[self.width, self.height] {
            writer.write(&(*dim as u16).to_le_bytes()).unwrap();
        }
        writer.write(&[1, 0]).unwrap();
        writer.write(&self.data).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::image13h::Image13h;

    // 3 by 2 image, we have 1 byte extra at the end to see if we ignore it correctly
    static GOOD_DATA: [u8; 13] = [3, 0, 2, 0, 1, 0, 1, 2, 3, 4, 5, 6, 7];

    #[test]
    fn test_not_enough_data_is_an_error() {
        for size in 0..12 {
            dbg!(size);
            assert!(Image13h::load(&mut &GOOD_DATA[0..size]).is_none());
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

        assert!(Image13h::load(&mut &bad_data1[..]).is_none());
        assert!(Image13h::load(&mut &bad_data2[..]).is_none());
        assert!(Image13h::load(&mut &bad_data3[..]).is_none());
    }

    #[test]
    fn test_loading_works() {
        let image13h = Image13h::load(&mut &GOOD_DATA[..]).unwrap();
        assert_eq!(image13h.width(), 3);
        assert_eq!(image13h.height(), 2);
        assert_eq!(image13h.line(0), [1, 2, 3]);
        assert_eq!(image13h.line(1), [4, 5, 6]);
    }

    #[test]
    fn test_saving_works() {
        let image13h = Image13h::load(&mut &GOOD_DATA[..]).unwrap();
        let mut buffer = Vec::new();
        image13h.save(&mut buffer);
        assert_eq!(buffer, &GOOD_DATA[0..buffer.len()]);
    }
}
