use std::convert::TryInto;
use std::io;

pub struct Sounddat {
    data: Vec<u8>,
    sizes: Vec<usize>,
    offsets: Vec<usize>,
}

impl Sounddat {
    pub fn load<T: io::Read>(reader: &mut T) -> Option<Sounddat> {
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

    pub fn sounds(&self) -> usize {
        self.sizes.len()
    }

    pub fn sound_data(&self, sound: usize) -> &[u8] {
        let offset = self.offsets[sound];
        &self.data[offset..offset + self.sizes[sound]]
    }
}

#[cfg(test)]
mod tests {
    use crate::sounddat::Sounddat;

    #[test]
    fn test_sounddat_loading_works() {
        let data = [1, 2, 3, 4, 5, 6, 4, 0, 0, 0, 2, 0, 0, 0];
        let sounddat = Sounddat::load(&mut data.as_ref()).unwrap();
        assert_eq!(sounddat.sounds(), 2);
        assert_eq!(sounddat.sound_data(0), [1, 2, 3, 4]);
        assert_eq!(sounddat.sound_data(1), [5, 6]);
    }
}
