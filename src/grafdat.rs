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
    items: Vec<image13h::Image13h>,
}

impl Grafdat {
    /// Load graf.dat from a reader. This function will return None if
    ///
    /// * The image can't be loaded
    /// * The image loaded is too small (see `MINIMUM_IMAGE_DIMENSIONS`)
    pub fn load<T: io::Read>(reader: T) -> Option<Grafdat> {
        match Grafdat::load_images(reader) {
            None => None,
            Some(images) => Some(Grafdat::load_from_images(&images)),
        }
    }

    /// Load graf.dat images from a reader. The error conditions of this function are the same
    /// as with `load`.
    pub fn load_images<T: io::Read>(mut reader: T) -> Option<Vec<image13h::Image13h>> {
        let w = IMAGE_DIMENSIONS.0;
        let h1 = FIRST_HALF_DIMENSIONS.1;
        let h2 = SECOND_HALF_DIMENSIONS.1;
        let h = h1 + h2;
        let first_half_size = w * h1;
        let second_half_size = w * h2;

        let mut data = vec![0; FILE_SIZE];
        if reader.read_exact(&mut data).is_err() {
            return None;
        }
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
        Some(images)
    }

    /// Load Grafdat from `IMAGES` Image13h images. Images need to have correct dimensions.
    fn load_from_images(images: &[image13h::Image13h]) -> Grafdat {
        assert_eq!(images.len(), IMAGES);
        for i in images {
            assert_eq!(i.width(), IMAGE_DIMENSIONS.0);
            assert_eq!(i.height(), IMAGE_DIMENSIONS.1);
        }
        let rects = get_image_rects();

        Grafdat {
            items: rects
                .into_iter()
                .map(|(index, rect)| images[index].subimage(&rect))
                .collect(),
        }
    }

    /// Create an empty Grafdat. All images are filled with color 0.
    pub fn empty() -> Grafdat {
        let images =
            vec![image13h::Image13h::empty(IMAGE_DIMENSIONS.0, IMAGE_DIMENSIONS.1); IMAGES];
        Grafdat::load_from_images(&images)
    }

    /// Save the Grafdat to a writer.
    pub fn save<T: io::Write>(&self, writer: T) {
        let images = self.to_images();
        Grafdat::save_images(&images, writer);
    }

    pub fn save_images<T: io::Write>(images: &[image13h::Image13h], mut writer: T) {
        assert_eq!(images.len(), IMAGES);
        let first_halves_filler = [0; SEGMENT_SIZE
            - image13h::HEADER_SIZE
            - FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1];
        let second_halves_filler = [0; SEGMENT_SIZE
            - image13h::HEADER_SIZE
            - SECOND_HALF_DIMENSIONS.0 * SECOND_HALF_DIMENSIONS.1];

        for image in images.iter() {
            writer.write_all(&[0; image13h::HEADER_SIZE]).unwrap();
            writer
                .write_all(&image.data()[0..FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1])
                .unwrap();
            writer.write_all(&first_halves_filler).unwrap();
        }
        for i in 0..IMAGES {
            // As mentioned in the module documentation images 9 and 10 have their second halves
            // swapped.
            let i = match i {
                9 => 10,
                10 => 9,
                _ => i,
            };
            writer.write_all(&[0; image13h::HEADER_SIZE]).unwrap();
            writer
                .write_all(&images[i].data()[FIRST_HALF_DIMENSIONS.0 * FIRST_HALF_DIMENSIONS.1..])
                .unwrap();
            writer.write_all(&second_halves_filler).unwrap();
        }
    }

    /// Convert the contents to graf.dat member images.
    pub fn to_images(&self) -> Vec<image13h::Image13h> {
        let mut images =
            vec![image13h::Image13h::empty(IMAGE_DIMENSIONS.0, IMAGE_DIMENSIONS.1); IMAGES];
        let rects = get_image_rects();
        for ((image_index, rect), item) in rects.iter().zip(self.items.iter()) {
            images[*image_index].blit(item, rect.left, rect.top);
        }
        images
    }

    /// Get a reference to the Grafdat items
    pub fn items(&self) -> &[image13h::Image13h] {
        &self.items
    }

    /// Get a mutable reference to the Grafdat items
    pub fn items_mut(&mut self) -> &mut [image13h::Image13h] {
        &mut self.items
    }

    pub fn main_menu(&self) -> &image13h::Image13h {
        // TODO think about addressing the problem of addressing the image pieces within the items
        // vector. Maybe change that to a record of some sort?
        self.items.last().unwrap()
    }
}

fn get_image_rects() -> Vec<(usize, image13h::Rect)> {
    // The result is a vector containing 2-tuples of (source image index, rect)
    let mouse = (1..13).map(|i| (3, (11 + (i - 1) * 16, 8, 11 + i * 16, 22)));

    let buttons = (0..14)
        .map(|i| (3, (11 + i * 16, 22, 27 + i * 16, 22 + 14)))
        .chain(vec![
            (3, (235 + 16, 8, 235 + 32, 8 + 14)),
            (3, (235, 8, 235 + 16, 8 + 14)),
        ]);
    let trees = (0..2)
        .flat_map(|i| {
            vec![
                (0, 0, 0, 0),
                (235, 22 + i * 56, 235 + 16, 36 + i * 56),
                (235 + 16, 22 + i * 56, 235 + 32, 36 + i * 56),
                (235, 36 + i * 56, 235 + 16, 50 + i * 56),
                (235 + 16, 36 + i * 56, 235 + 32, 50 + i * 56),
                (235, 50 + i * 56, 235 + 16, 64 + i * 56),
                (235 + 16, 50 + i * 56, 235 + 32, 64 + i * 56),
                (235 + 16, 64 + i * 56, 235 + 32, 78 + i * 56),
            ]
        })
        .map(|coords| (3, coords));

    let dead = (0..3).map(|i| {
        (
            3,
            (
                11 + 13 * 16,
                8 + (3 + i) * 14,
                27 + 13 * 16,
                8 + (4 + i) * 14,
            ),
        )
    });

    let hit = vec![
        (3, (11 + 12 * 16, 8, 27 + 12 * 16, 21)),
        (3, (11 + 13 * 16, 8, 27 + 13 * 16, 21)),
    ];

    let pictures = (0..3) // Grass 0..9
        .map(|i| (3, (11 + 16 * i, 36, 27 + 16 * i, 50)))
        .chain((0..6).map(|i| (4, (303, (2 + i) * 14, 319, (3 + i) * 14))))
        // Rocks 9..22
        .chain((0..13).map(|i| (3, (11 + 16 * i, 92, 27 + 16 * i, 106))))
        // Dry earth, 22..25
        .chain((0..3).map(|i| (3, (235, 134 + (i * 19), 257, 152 + (i * 19)))))
        // Roads 25..46
        .chain((3..13).map(|i| (3, (11 + 16 * i, 8 + 126, 27 + 16 * i, 8 + 140))))
        .chain((0..11).map(|i| (3, (11 + 16 * i, 8 + 140, 27 + 16 * i, 8 + 154))))
        // Bridges 46..54
        .chain((0..8).map(|i| (3, (11 + 16 * i, 8 + 154, 27 + 16 * i, 8 + 168))))
        // Gadgets 54..65
        .chain((3..14).map(|i| (3, (11 + 16 * i, 36, 27 + 16 * i, 50))))
        // Gadgets 65..68
        .chain((0..3).map(|i| {
            (
                3,
                (11 + 16 * (i + 11), 8 + 140, 27 + 16 * (i + 11), 22 + 140),
            )
        }))
        // Gadgets 68..74
        .chain((0..6).map(|i| (3, (11 + 16 * (i + 8), 8 + 154, 27 + 16 * (i + 8), 22 + 154))))
        // Water 74..113
        .chain((0..13).map(|i| (3, (11 + 16 * i, 50, 27 + 16 * i, 64))))
        .chain((0..13).map(|i| (3, (11 + 16 * i, 64, 27 + 16 * i, 78))))
        .chain((0..13).map(|i| (3, (11 + 16 * i, 78, 27 + 16 * i, 92))))
        // Trees 113..127
        .chain((0..7).map(|i| (3, (11 + 32 * i, 8 + (8 * 14), 43 + 32 * i, 8 + (9 * 14)))))
        .chain((0..7).map(|i| (3, (11 + 32 * i, 8 + (7 * 14), 43 + 32 * i, 8 + (8 * 14)))))
        // Buildings being built 127..136
        .chain((0..9).map(|i| (7, (i * 16, 12 * 14, (i + 1) * 16, 13 * 14))))
        // NOTE: 1-element hole here
        .chain(vec![(3, (0, 0, 1, 1))])
        // Buildings 137..257
        .chain((0..6).flat_map(|j| {
            (0..20)
                .map(|i| {
                    (
                        7,
                        (
                            i * 16,
                            (j + 6) * 14,
                            (i + 1) * 16
                                // We want to skip the last column when accessing the right-most
                                // rects, because the naive non-conditional formula spans one pixel
                                // too far to the right.
                                - match i {
                                    19 => 1,
                                    _ => 0,
                                },
                            (j + 7) * 14,
                        ),
                    )
                })
                // TODO Is this collect() here really necessary? This code smells bad, but the
                // naive approach results in "the closure may outlive j" issue.
                .collect::<Vec<_>>()
        }))
        // Ruins 257..266
        .chain((0..9).map(|i| (7, (i * 16, 13 * 14, (i + 1) * 16, 14 * 14))))
        // Palisade 266..278
        .chain((0..12).map(|i| (4, (303 - 16, i * 14, 319 - 16, (i + 1) * 14))))
        // Shields 278..282
        .chain((0..4).map(|i| (4, (303, (8 + i) * 14, 319, (9 + i) * 14))))
        // Healing 282..284
        .chain((0..2).map(|i| (4, (303, i * 14, 319, (1 + i) * 14))));

    let fire = (0..14).map(|i| (3, (11 + 16 * i, 8 + 168, 27 + 16 * i, 8 + 168 + 14)));
    let borders = vec![
        (3, (0, 0, 11, 197)),
        (3, (0, 0, 268, 8)),
        (3, (267, 0, 274, 199)),
        // NOTE: the original had the following coordinates, the one line going beyond what's in
        // GRAF.DAT was filled with black.
        //(3, (0, 190, 268, 200)),
        (3, (0, 190, 268, 199)),
    ];
    let wood = vec![
        (3, (272, 7, 276, 28)),
        (3, (278, 7, 310, 28)),
        (3, (299, 9, 314, 150)),
    ];
    let second_buttons = vec![
        // TODO In the original game the first two buttons have a bar drawn over them after
        // loading from disk and before extracting from the bigger image. See if this is
        // necesary and, if so replicate that here.
        (7, (108, 142, 219, 160)),
        (7, (108, 114, 219, 132)),
        (3, (274, 38, 292, 54)),
        (3, (274, 58, 292, 74)),
    ];
    let screens = vec![
        // Main menu
        (2, (0, 0, 319, 199)),
    ];

    // TODO Movers, Shadow, Missiles

    // tuples in coords are of form (x1, y1, x2, y2) like in GetImage13h in the original game.
    // x1 and y1 are inclusive, x1 and y2 are exclusive.
    let indexes_coords = mouse
        .chain(buttons)
        .chain(trees)
        .chain(dead)
        .chain(hit)
        .chain(pictures)
        .chain(fire)
        .chain(borders)
        .chain(wood)
        .chain(second_buttons)
        .chain(screens);
    indexes_coords
        .map(|(index, (x1, y1, x2, y2))| (index, image13h::Rect::from_ranges(x1..x2, y1..y2)))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::grafdat::{Grafdat, IMAGES, IMAGE_DIMENSIONS, SEGMENT_SIZE};
    use crate::image13h;

    fn dummy_graf_dat_content() -> Vec<u8> {
        (0..IMAGES)
            .flat_map(|i| vec![i as u8; SEGMENT_SIZE])
            .chain((0..IMAGES).flat_map(|i| {
                vec![
                    match i {
                        9 => 10,
                        10 => 9,
                        other => other,
                    } as u8;
                    SEGMENT_SIZE
                ]
            }))
            .collect()
    }

    #[test]
    fn test_loading_and_saving_images_works() {
        let loaded1 = Grafdat::load_images(&dummy_graf_dat_content()[..]).unwrap();
        let expected_images = (0..IMAGES)
            .map(|i| {
                image13h::Image13h::filled_with_color(
                    IMAGE_DIMENSIONS.0,
                    IMAGE_DIMENSIONS.1,
                    i as u8,
                )
            })
            .collect::<Vec<_>>();
        // First let's verify that after loading from disk we get the expected images:
        assert_eq!(loaded1, expected_images);
        // ...then make sure that after we can save without crashing. In the process of saving
        // we'll discard some data so we can't directly compare the output with the dummy data we
        // prepared initially.
        let mut saved1 = Vec::new();
        Grafdat::save_images(&loaded1, &mut saved1);
        // Now, saved1 should only contain data that actually matters. Of we load from it we should
        // get the same images as before:
        let loaded2 = Grafdat::load_images(&saved1[..]).unwrap();
        assert_eq!(loaded2, loaded1);
        // And now when we save that we expect the output to stay the same as the previous saving
        // result:
        let mut saved2 = Vec::new();
        Grafdat::save_images(&loaded2, &mut saved2);
        assert_eq!(saved2, saved1);
    }

    #[test]
    fn test_loading_and_saving_does_not_crash() {
        // It's too involved (at least for now) to test that we load *exactly* the pixels we want
        // using precisely the rects we want to, so I think the next best thing is to test that we
        // can load graf.dat without crashing, we can save it (discaring some irrelevant data),
        // load it again, check if the loaded results are the same as the first time and, in the
        // very end, save again and compare the result with the output of the first save.
        let grafdat1 = Grafdat::load(&dummy_graf_dat_content()[..]).unwrap();
        let mut saved1 = Vec::new();
        grafdat1.save(&mut saved1);
        let grafdat2 = Grafdat::load(&saved1[..]).unwrap();
        assert_eq!(grafdat2, grafdat1);
        let mut saved2 = Vec::new();
        grafdat2.save(&mut saved2);
        assert_eq!(saved2, saved1);
    }
}
