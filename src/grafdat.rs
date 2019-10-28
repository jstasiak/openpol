//! graf.dat data access operations
//!
//! # graf.dat file format
//!
//! The file contains 30 [image13h](../image13h/index.html) images stored at 33 000 bytes
//! increments. The 6-byte headers present are invalid – they need to be ignored. The
//! dimensions of the images are hardcoded: the first 15 images are 319x100 pixels, the next
//! 15 images are 319x99 pixels. The remaining space within a segment (33 000 - 6 bytes
//! header - 319*100 (or 99) bytes for pixel data) is not used.
//!
//! The 30 images in the file are actually 15 images, just split in "halves" (the second "half"
//! being 1 pixel shorter): logical image consist of images i and i + 15. There's an exception to
//! this rule: images 9 and 10 (0-based) have their second halves swapped.

use crate::image13h;
use std::io;

pub const SEGMENT_SIZE: usize = 33_000;
pub const SEGMENTS: usize = 30;
pub const FILE_SIZE: usize = SEGMENT_SIZE * SEGMENTS;
pub const IMAGES: usize = 15;
pub const FIRST_HALF_DIMENSIONS: (usize, usize) = (319, 100);
pub const SECOND_HALF_DIMENSIONS: (usize, usize) = (319, 99);
pub const IMAGE_DIMENSIONS: (usize, usize) = (319, 199);

#[derive(Debug, Eq, PartialEq)]
pub struct Grafdat {
    images: Vec<image13h::Image13h>,
}

impl Grafdat {
    /// Create a new empty Grafdat (all images are filled with color 0).
    pub fn empty() -> Grafdat {
        Grafdat {
            images: vec![image13h::Image13h::empty(IMAGE_DIMENSIONS.0, IMAGE_DIMENSIONS.1); IMAGES],
        }
    }

    /// Load graf.dat from a reader. This function will return None if
    ///
    /// * The image can't be loaded
    /// * The image loaded is too small (see `MINIMUM_IMAGE_DIMENSIONS`)
    pub fn load<T: io::Read>(mut reader: T) -> Option<Grafdat> {
        let w = IMAGE_DIMENSIONS.0;
        let h1 = FIRST_HALF_DIMENSIONS.1;
        let h2 = SECOND_HALF_DIMENSIONS.1;
        let h = h1 + h2;
        let first_half_size = w * h1;
        let second_half_size = w * h2;

        let mut data = vec![0; FILE_SIZE];
        match reader.read_exact(&mut data) {
            Err(_) => return None,
            Ok(_) => (),
        };
        let mut images = Vec::new();
        for i in 0..IMAGES {
            let mut image = image13h::Image13h::empty(w, h);
            let image_data = image.data_mut();
            let offset1 = i * SEGMENT_SIZE + image13h::HEADER_SIZE;
            let offset2 = match i {
                // Images 9 and 10 have their second halves swapped, we need to handle this
                // manually.
                9 => 25,
                10 => 24,
                _ => i + IMAGES,
            } * SEGMENT_SIZE
                + image13h::HEADER_SIZE;
            let src1 = &data[offset1..offset1 + first_half_size];
            let src2 = &data[offset2..offset2 + second_half_size];
            image_data[0..first_half_size].copy_from_slice(src1);
            image_data[first_half_size..first_half_size + second_half_size].copy_from_slice(src2);
            images.push(image);
        }
        Some(Grafdat::load_from_images(images))
    }

    /// Load Grafdat from `IMAGES` Image13h images. Images need to have correct dimensions.
    pub fn load_from_images(images: Vec<image13h::Image13h>) -> Grafdat {
        assert_eq!(images.len(), IMAGES);
        for i in &images {
            assert_eq!(i.width(), IMAGE_DIMENSIONS.0);
            assert_eq!(i.height(), IMAGE_DIMENSIONS.1);
        }
        Grafdat { images }
    }

    /// Save the Grafdat to a writer.
    pub fn save<T: io::Write>(&self, mut writer: T) {
        let first_halves_filler = [0; SEGMENT_SIZE
            - image13h::HEADER_SIZE
            - FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1];
        let second_halves_filler = [0; SEGMENT_SIZE
            - image13h::HEADER_SIZE
            - SECOND_HALF_DIMENSIONS.0 * SECOND_HALF_DIMENSIONS.1];

        let images = self.to_images();
        for i in 0..IMAGES {
            writer.write(&[0; image13h::HEADER_SIZE]).unwrap();
            writer
                .write(&images[i].data()[0..FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1])
                .unwrap();
            writer.write(&first_halves_filler).unwrap();
        }
        for i in 0..IMAGES {
            // As mentioned in the module documentation images 9 and 10 have their second halves
            // swapped.
            let i = match i {
                9 => 10,
                10 => 9,
                _ => i,
            };
            writer.write(&[0; image13h::HEADER_SIZE]).unwrap();
            writer
                .write(&images[i].data()[FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1..])
                .unwrap();
            writer.write(&second_halves_filler).unwrap();
        }
    }

    /// Convert Grafdat to a slice of `IMAGES` Image13h images.
    pub fn to_images(&self) -> &[image13h::Image13h] {
        &self.images
    }

    /*
    /// Save the font to a writer.
    pub fn save<T: io::Write>(&self, writer: T) {}

    pub fn empty(width: usize, height: usize) -> Grafdat {
        Grafdat {
            images: vec![image13h::Image13h::empty(319, 199); 15],
        }
    }
    */
}

#[cfg(test)]
mod tests {
    use crate::grafdat::{Grafdat, IMAGES, IMAGE_DIMENSIONS};
    use crate::image13h;
    use std::fs;

    #[test]
    fn test_loading_and_saving_works() {
        let dummy_graf_dat_content = fs::read("dummy_graf.dat").unwrap();
        let grafdat = Grafdat::load(&dummy_graf_dat_content[..]).unwrap();
        let mut images = Vec::new();
        for i in 0..IMAGES {
            images.push(image13h::Image13h::filled_with_color(
                IMAGE_DIMENSIONS.0,
                IMAGE_DIMENSIONS.1,
                i as u8,
            ));
        }
        let expected_grafdat = Grafdat::load_from_images(images);
        // First let's verify that after loading from disk we get the expected images
        assert_eq!(grafdat, expected_grafdat);
        // ...then make sure that after saving grafdat we get the exact save binary content.
        let mut buf = Vec::new();
        grafdat.save(&mut buf);

        // NOTE: Some of the content is irrelevant: image13h headers and data between segments but
        // this comparison doesn't account for this at the moment.
        assert_eq!(buf, dummy_graf_dat_content);
    }
}
