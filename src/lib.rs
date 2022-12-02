use std::{cell::Cell, f32::consts::TAU};

use az::WrappingCast;
use gdnative::{
    api::{
        audio_stream_sample::LoopMode, AudioStreamGenerator, AudioStreamGeneratorPlayback,
        AudioStreamPlayer, AudioStreamPlayer3D, AudioStreamSample, Resource,
    },
    export::user_data::Map,
    prelude::*,
};

#[derive(NativeClass)]
#[inherit(Resource)]
pub struct HelloWorld;

#[methods]
impl HelloWorld {
    fn new(_base: &Resource) -> Self {
        HelloWorld
    }

    #[method]
    fn _ready(&self, #[base] _base: &Resource) {
        godot_print!("Hello, world.");
    }
}

/// Music provided by the game.
#[derive(NativeClass)]
#[inherit(Resource)]
pub struct ProvidedMusicClip {
    #[property]
    audio_stream: Ref<AudioStreamSample>,
    #[property]
    samples_per_beat: u32,
    #[property]
    first_beat_sample: u32,
}

#[methods]
impl ProvidedMusicClip {
    pub fn new(_base: &Resource) -> Self {
        Self {
            audio_stream: AudioStreamSample::new().into_shared(),
            samples_per_beat: 22050,
            first_beat_sample: 0,
        }
    }

    #[method]
    pub fn as_music_clip(&self) -> Instance<MusicClip> {
        let mut float_data = Vec::new();
        let audio_stream = unsafe { self.audio_stream.assume_safe() };
        let pcm_data = audio_stream.data();
        for sample_index in 0..pcm_data.len() / 4 {
            let [b0, b1, b2, b3] = [
                pcm_data.get(sample_index * 4 + 0),
                pcm_data.get(sample_index * 4 + 1),
                pcm_data.get(sample_index * 4 + 2),
                pcm_data.get(sample_index * 4 + 3),
            ];
            float_data.push((pcm_sample_to_float(b0, b1), pcm_sample_to_float(b2, b3)))
        }
        Instance::emplace(MusicClip {
            audio_data: float_data,
            // Make a new one so that we don't stomp over the original when this clip gets edited.
            audio_stream: make_default_audio_stream(),
            audio_stream_dirty: true,
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: self.first_beat_sample,
        })
        .into_shared()
    }
}

#[derive(NativeClass)]
#[inherit(Resource)]
pub struct MusicClip {
    audio_data: Vec<(f32, f32)>,
    #[property]
    audio_stream: Ref<AudioStreamSample>,
    audio_stream_dirty: bool,
    #[property]
    samples_per_beat: u32,
    #[property]
    first_beat_sample: u32,
}

fn make_default_audio_stream() -> Ref<AudioStreamSample> {
    let data = ByteArray::new();
    let audio_stream = AudioStreamSample::new();
    audio_stream.set_format(AudioStreamSample::FORMAT_16_BITS);
    audio_stream.set_mix_rate(44100);
    audio_stream.set_stereo(true);
    audio_stream.set_loop_mode(AudioStreamSample::LOOP_DISABLED);
    audio_stream.set_data(data);
    audio_stream.into_shared()
}

#[methods]
impl MusicClip {
    pub fn new(_base: &Resource) -> Self {
        Self {
            audio_data: vec![],
            audio_stream: make_default_audio_stream(),
            audio_stream_dirty: false,
            samples_per_beat: 22050,
            first_beat_sample: 0,
        }
    }

    #[method]
    fn num_samples(&self) -> usize {
        self.audio_data.len()
    }

    #[method]
    fn beat_time(&self) -> f64 {
        self.samples_per_beat as f64 / 44100.0
    }

    #[method]
    fn set_beat_time(&mut self, beat_time: f64) {
        self.samples_per_beat = (beat_time * 44100.0) as _;
    }

    #[method]
    fn start_time(&self) -> f64 {
        self.first_beat_sample as f64 / 44100.0
    }

    #[method]
    fn duration(&self) -> f64 {
        self.num_samples() as f64 / 44100.0
    }

    #[method]
    fn beats(&self) -> f64 {
        (self.num_samples() - self.first_beat_sample as usize) as f64 / self.samples_per_beat as f64
    }

    #[method]
    fn beat_to_sample(&self, beat: f64) -> usize {
        (self.samples_per_beat as f64 * beat) as usize + self.first_beat_sample as usize
    }

    #[method]
    fn play_audio(&mut self, output: Ref<AudioStreamPlayer3D>) {
        if self.audio_stream_dirty {
            self.audio_stream_dirty = false;
            let mut data = ByteArray::new();
            for &(left, right) in &self.audio_data {
                let (a, b) = float_sample_to_pcm(left);
                data.push(a);
                data.push(b);
                let (a, b) = float_sample_to_pcm(right);
                data.push(a);
                data.push(b);
            }
            unsafe { self.audio_stream.assume_safe() }.set_data(data);
        }
        let output = unsafe { output.assume_safe() };
        output.set_stream(&self.audio_stream)
    }

    #[method]
    fn set_looping(&self, looping: bool) {
        let audio = unsafe { self.audio_stream.assume_safe() };
        audio.set_loop_begin(self.first_beat_sample as _);
        let duration = self.num_samples() - self.first_beat_sample as usize;
        let end = self.first_beat_sample as usize + duration
            - duration % (self.samples_per_beat as usize / 8);
        audio.set_loop_end(end as _);
        audio.set_loop_mode(if looping {
            LoopMode::FORWARD.0
        } else {
            LoopMode::DISABLED.0
        });
    }

    #[method]
    fn is_looping(&self) -> bool {
        unsafe { self.audio_stream.assume_safe() }.loop_mode() != LoopMode::DISABLED
    }

    #[method]
    fn trim(&self, start: f64, end: f64) -> Instance<Self, Unique> {
        let start_sample = self.beat_to_sample(start);
        let end_sample = self.beat_to_sample(end);
        let audio_data = Vec::from(&self.audio_data[start_sample..end_sample]);
        Instance::emplace(Self {
            audio_data,
            audio_stream: make_default_audio_stream(),
            audio_stream_dirty: true,
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: 0,
        })
    }

    #[method]
    fn delay(&self) -> Instance<Self, Unique> {
        let step = self.samples_per_beat as usize * 3 / 4;
        let decay = 0.4;
        let mut audio_data: Vec<_> = self
            .audio_data
            .iter()
            .map(|x| (x.0 * (1.0 - decay), x.1 * (1.0 - decay)))
            .collect();
        for sample in step..self.audio_data.len() {
            let prev = audio_data[sample - self.samples_per_beat as usize * 3 / 4];
            audio_data[sample].0 += prev.0 * decay;
            audio_data[sample].1 += prev.1 * decay;
        }
        Instance::emplace(Self {
            audio_data,
            audio_stream: make_default_audio_stream(),
            audio_stream_dirty: true,
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: 0,
        })
    }

    #[method]
    fn distort(&self) -> Instance<Self, Unique> {
        let map = |x: f32| {
            let e3x = (x * 3.0).exp();
            2.0 * (e3x / (e3x + 1.0)) - 1.0
        };
        let audio_data = self
            .audio_data
            .iter()
            .map(|x| (map(x.0), map(x.1)))
            .collect();
        Instance::emplace(Self {
            audio_data,
            audio_stream: make_default_audio_stream(),
            audio_stream_dirty: true,
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: 0,
        })
    }

    #[method]
    fn write_sample(
        &mut self,
        target_start: f64,
        source: Instance<MusicClip>,
        source_start: f64,
        source_end: f64,
        pitch: f32,
    ) {
        let target_start = self.beat_to_sample(target_start);
        let source = unsafe { source.assume_safe() };
        let source_start = source
            .map(|source, _res| source.beat_to_sample(source_start))
            .unwrap();
        let source_end = source
            .map(|source, _res| source.beat_to_sample(source_end))
            .unwrap();
        source
            .map(|source, _res| {
                let source_audio = &source.audio_data;
                let target_audio = &mut self.audio_data;

                let source_len = source_end - source_start;
                let source_len = source_len.min(source.num_samples() - source_start - 1);
                let target_len = (source_len as f32 / pitch) as usize;
                let target_end = target_start + target_len;

                if target_audio.len() < target_end {
                    target_audio.resize(target_end, (0.0, 0.0));
                }

                for offset in 0..target_len {
                    let source_index = source_start as f32 + (offset as f32 * pitch);
                    let target_index = target_start + offset;
                    target_audio[target_index] = lerp_samples(
                        source_audio[source_index.floor() as usize],
                        source_audio[source_index.ceil() as usize],
                        source_index % 1.0,
                    );
                }
            })
            .unwrap();
        self.audio_stream_dirty = true;
    }

    #[method]
    fn mixdown(&mut self, sources: Vec<Instance<MusicClip>>, start: f64, end: f64) {
        assert!(sources.len() > 0);
        self.extend(end);
        let start = self.beat_to_sample(start);
        let end = self.beat_to_sample(end);
        let source_refs: Vec<_> = sources.iter().map(|s| unsafe { s.assume_safe() }).collect();
        let samples_per_beat = source_refs[0].map(|clip, _| clip.samples_per_beat).unwrap();
        for source in &source_refs {
            source
                .map(|clip, _| {
                    assert_eq!(clip.samples_per_beat, samples_per_beat);
                    for index in start..end.min(clip.audio_data.len()) {
                        let source = clip.audio_data[index];
                        let target = &mut self.audio_data;
                        target[index].0 += source.0;
                        target[index].1 += source.1;
                    }
                })
                .unwrap();
        }
        self.audio_stream_dirty = true;
    }

    #[method]
    fn extend(&mut self, duration: f64) {
        let duration = self.beat_to_sample(duration);
        let target_audio = &mut self.audio_data;

        if target_audio.len() < duration {
            target_audio.resize(duration, (0.0, 0.0));
        }
    }

    #[method]
    fn write_silence(&mut self, start: f64, end: f64) {
        let start = self.beat_to_sample(start);
        let end = self.beat_to_sample(end);

        let target_len = end - start;
        let target_audio = &mut self.audio_data;

        if target_audio.len() < end {
            target_audio.resize(end, (0.0, 0.0));
        }

        for offset in 0..target_len {
            let target_index = start + offset;
            target_audio[target_index] = (0.0, 0.0);
        }

        self.audio_stream_dirty = true;
    }

    #[method]
    fn clear(&mut self) {
        self.audio_data.clear();
        self.audio_stream_dirty = true;
    }
}

fn pcm_sample_to_float(byte0: u8, byte1: u8) -> f32 {
    let byte1: i8 = byte1.wrapping_cast();
    byte1 as f32 / 128.0 + byte0 as f32 / (65536.0 / 2.0)
}

fn float_sample_to_pcm(sample: f32) -> (u8, u8) {
    let as_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
    let [a, b] = as_i16.to_le_bytes();
    (a, b)
}

fn lerp_samples(a: (f32, f32), b: (f32, f32), f: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}

fn init(handle: InitHandle) {
    handle.add_class::<MusicClip>();
    handle.add_class::<ProvidedMusicClip>();
}

godot_init!(init);
