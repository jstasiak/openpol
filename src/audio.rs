use std::sync::Arc;

pub struct Sound {
    data: Arc<Vec<u8>>,
}

impl Sound {
    pub fn new(data: Vec<u8>) -> Sound {
        Sound {
            data: Arc::new(data),
        }
    }

    /**
     * Produce a rodio Source that plays samples from this Sound object.
     *
     * The source is cheap to create (it only references the data, doesn't own it, there are
     * no copies created).
     */
    pub fn as_source(&self) -> RodioSource {
        RodioSource::new(self.data.clone())
    }
}

pub struct RodioSource {
    data: Arc<Vec<u8>>,
    position: usize,
}

impl RodioSource {
    pub fn new(data: Arc<Vec<u8>>) -> RodioSource {
        RodioSource { data, position: 0 }
    }
}

// rodio Sources are Iterators of Samples
impl Iterator for RodioSource {
    // rodio doesn't support u8 (the format we get from Sounddat) so we choose the next best thing.
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.data.len() {
            // Since the samples we have are u8 and we need to produce u16 we just multiply.
            let sample = Some((self.data[self.position] as u16) << 8);
            self.position += 1;
            sample
        } else {
            None
        }
    }
}

impl rodio::Source for RodioSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.data.len() - self.position)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        // The sample rate the original game and the data files use.
        22_050
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        // We *could* calculate this if we really wanted but we don't have to, so...
        None
    }
}
