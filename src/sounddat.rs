//! sound.dat data access operations.
//!
//! # sound.dat file format
//! The file consists of concatenated N sounds followed by N little endian 32-bit unsigned
//! integer sizes (in bytes). The nth size corresponds to the nth sound, the file layout
//! looks like this:
//!
//! `[ 0th sound ] [ 1st sound ] ... [ N-2th sound ] [N-1th sound]
//! [ 0th size ] [ 1st size ] ... [ N-2th size ] [ N-1th size]`
//!
//! The original game hardcodes the number of sounds when opening the sound.dat file, but it's
//! possible to autodect it. This module performs autodetection like this: read 4-byte integers
//! starting with the very end of the file and add them together until the sum is equal to
//! B - 4 * N (where B is total file size in bytes and N is the number of sizes read so far).
//!
//! The algorithm has been verified with sound.dat coming from the CD version of Polanie
//! (SHA1 hash `8033978a51c176122ba507e417e8d758fdaa70a9`, 3 681 170 bytes) - the file contains 183 sounds.
//!
//! # Sound format
//!
//! The individual sounds are unsigned bytes containing single channel of 22 050Hz-sampled raw audio data.
//!
//! # Example
//!
//! An `openpol-extract-audio` sample binary which uses this code is provided. You can listen to
//! a chosen sound using sox and mpv like this:
//!
//! `sox -r22050 -t ub -c 1 <(cargo run --bin openpol-extract-audio -- SOUND.DAT 20) -t wav - | mpv -`
use std::convert::TryInto;
use std::io;

/// A way to access sound.dat contents.
pub struct Sounddat {
    data: Vec<u8>,
    sizes: Vec<usize>,
    offsets: Vec<usize>,
}

impl Sounddat {
    /// Load sound.dat contents. All of it is read into memory.
    ///
    /// # Errors
    /// The code will panic if `reader` cannot read to end. If the number of sounds can't be
    /// autodetected (the file contains unexpected data) the function will return `None`.
    pub fn load<T: io::Read>(mut reader: T) -> Option<Sounddat> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();

        let total_bytes = data.len();
        let mut accumulator = 0usize;
        const ENTRY_SIZE: usize = 4;
        let mut sounds = 0;
        let mut data_bytes = total_bytes;
        let mut sizes = Vec::new();

        loop {
            let offset = total_bytes - ENTRY_SIZE * (sounds + 1);
            let entry =
                u32::from_le_bytes(data[offset..offset + ENTRY_SIZE].try_into().unwrap()) as usize;
            data_bytes -= ENTRY_SIZE;
            sounds += 1;
            sizes.push(entry);
            accumulator += entry;
            if accumulator > data_bytes {
                return None;
            }
            if accumulator == data_bytes {
                break;
            }
        }

        sizes.reverse();
        let mut offsets = Vec::new();
        let mut offset = 0;
        for size in &sizes {
            offsets.push(offset);
            offset += size;
        }

        Some(Sounddat {
            data,
            sizes,
            offsets,
        })
    }

    /// The number of sounds in the file.
    pub fn sounds(&self) -> usize {
        self.sizes.len()
    }

    /// The `sound`'s data (`sound` is 0-based). The data is to be interpreted as described by the
    /// [module's documentation on the sound format](index.html#sound-format).
    pub fn sound_data(&self, sound: usize) -> &[u8] {
        let offset = self.offsets[sound];
        &self.data[offset..offset + self.sizes[sound]]
    }

    /// Convert the structure into a vector of buffers containing the pieces of data.
    pub fn into_vecs(self) -> Vec<Vec<u8>> {
        let mut vecs = Vec::new();
        let mut rest = self.data;
        for size in self.sizes {
            let mut chunk = rest;
            rest = chunk.split_off(size);
            vecs.push(chunk);
        }
        vecs
    }
}

#[cfg(test)]
mod tests {
    use crate::sounddat::Sounddat;

    #[test]
    fn test_sounddat_loading_works() {
        let data = [1, 2, 3, 4, 5, 6, 4, 0, 0, 0, 2, 0, 0, 0];
        let sounddat = Sounddat::load(&data[..]).unwrap();
        assert_eq!(sounddat.sounds(), 2);
        assert_eq!(sounddat.sound_data(0), [1, 2, 3, 4]);
        assert_eq!(sounddat.sound_data(1), [5, 6]);
    }
}
