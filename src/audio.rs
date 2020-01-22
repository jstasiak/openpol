use sdl2::audio::{AudioCallback, AudioDevice};
use std::cmp;

pub struct Audio {
    pub data: Vec<u8>,
    pub position: usize,
    pub silence: u8,
}

impl AudioCallback for Audio {
    type Channel = u8;

    fn callback(&mut self, out: &mut [u8]) {
        let to_buffer = cmp::min(out.len(), self.data.len() - self.position);
        out[..to_buffer].copy_from_slice(&self.data[self.position..self.position + to_buffer]);
        self.position += to_buffer;
        // TODO repeat the audio just like we repeat the video. Going silent after the first play
        // for now.
        if self.position == self.data.len() {
            for x in out[to_buffer..].iter_mut() {
                *x = self.silence;
            }
        }
    }
}

pub fn clear_audio(audio_device: &mut AudioDevice<Audio>) {
    let mut audio_lock = audio_device.lock();
    audio_lock.data = Vec::new();
    audio_lock.position = 0;
}
