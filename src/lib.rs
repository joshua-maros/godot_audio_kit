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

#[derive(NativeClass)]
#[inherit(Resource)]
pub struct MusicClip {
    #[property]
    audio: Ref<AudioStreamSample>,
    samples: Cell<Option<i32>>,
    #[property]
    samples_per_beat: i32,
    #[property]
    first_beat_sample: i32,
}

#[methods]
impl MusicClip {
    pub fn new(_base: &Resource) -> Self {
        let data = ByteArray::new();
        let audio = AudioStreamSample::new();
        audio.set_format(AudioStreamSample::FORMAT_16_BITS);
        audio.set_mix_rate(44100);
        audio.set_stereo(true);
        audio.set_loop_mode(AudioStreamSample::LOOP_DISABLED);
        audio.set_data(data);
        let audio = audio.into_shared();
        Self {
            audio,
            samples: Cell::new(None),
            samples_per_beat: 22050,
            first_beat_sample: 0,
        }
    }

    #[method]
    fn num_samples(&self) -> i32 {
        if let Some(samples) = self.samples.get() {
            samples
        } else {
            let audio = unsafe { self.audio.assume_safe() };
            let samples = audio.data().len() / 4;
            self.samples.set(Some(samples));
            samples
        }
    }

    #[method]
    fn beat_time(&self) -> f64 {
        self.samples_per_beat as f64 / 44100.0
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
        (self.num_samples() - self.first_beat_sample) as f64 / self.samples_per_beat as f64
    }

    #[method]
    fn beat_to_sample(&self, beat: f64) -> i32 {
        (self.samples_per_beat as f64 * beat) as i32 + self.first_beat_sample
    }

    #[method]
    fn play_audio(&self, output: Ref<AudioStreamPlayer3D>) {
        let output = unsafe { output.assume_safe() };
        output.set_stream(&self.audio)
    }

    #[method]
    fn set_looping(&self, looping: bool) {
        let audio = unsafe { self.audio.assume_safe() };
        audio.set_loop_begin(self.first_beat_sample as _);
        let duration = self.num_samples() - self.first_beat_sample;
        let end = self.first_beat_sample + duration - duration % self.samples_per_beat;
        audio.set_loop_end(end as _);
        audio.set_loop_mode(if looping {
            LoopMode::FORWARD.0
        } else {
            LoopMode::DISABLED.0
        });
    }

    #[method]
    fn is_looping(&self) -> bool {
        unsafe { self.audio.assume_safe() }.loop_mode() != LoopMode::DISABLED
    }

    #[method]
    fn trim(&self, start: f64, end: f64) -> Instance<Self, Unique> {
        let start_sample = self.beat_to_sample(start);
        let end_sample = self.beat_to_sample(end);
        let mut data = ByteArray::new();
        let audio = unsafe { self.audio.assume_safe() };
        let pcm_data = audio.data();
        for i in start_sample * 4..end_sample * 4 {
            assert!(i < pcm_data.len());
            data.push(pcm_data.get(i));
        }
        let len = data.len();
        let result = AudioStreamSample::new();
        result.set_format(AudioStreamSample::FORMAT_16_BITS);
        result.set_mix_rate(44100);
        result.set_stereo(true);
        result.set_loop_mode(AudioStreamSample::LOOP_DISABLED);
        result.set_data(data);
        Instance::emplace(Self {
            audio: result.into_shared(),
            samples: Cell::new(Some(len / 4)),
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: 0,
        })
    }

    #[method]
    fn write_sample(
        &self,
        target_start: f64,
        source: Instance<MusicClip>,
        source_start: f64,
        source_end: f64,
        pitch: f64,
    ) {
        assert_eq!(pitch, 1.0);
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
                let source_audio = unsafe { source.audio.assume_safe() };
                let target_audio = unsafe { self.audio.assume_safe() };
                let mut target_data = target_audio.data();
                let source_data = source_audio.data();

                let source_len = source_end - source_start;
                let source_len = source_len.min(source_data.len() / 4 - source_start);
                let target_len = source_len;
                let target_end = target_start + target_len;

                while target_data.len() < target_end * 4 {
                    target_data.push(0);
                }
                debug_assert!(target_data.len() >= target_end * 4);

                for offset in 0..target_len {
                    let source_index = source_start + offset;
                    let target_index = target_start + offset;
                    for byte in 0..4 {
                        debug_assert!(target_index * 4 + byte < target_end * 4);
                        debug_assert!(source_index * 4 + byte < source_end * 4);
                        target_data.set(
                            target_index * 4 + byte,
                            source_data.get(source_index * 4 + byte),
                        );
                    }
                }
                self.samples.set(Some(target_data.len() / 4));
                target_audio.set_data(target_data);
            })
            .unwrap();
        godot_print!("alskjdflaksjdf");
    }

    #[method]
    fn write_silence(&self, start: f64, end: f64) {
        let start = self.beat_to_sample(start);
        let end = self.beat_to_sample(end);
        let target_audio = unsafe { self.audio.assume_safe() };
        let mut target_data = target_audio.data();

        let target_len = end - start;

        while target_data.len() < end * 4 {
            target_data.push(0);
        }
        debug_assert!(target_data.len() >= end * 4);

        for offset in 0..target_len {
            let target_index = start + offset;
            for byte in 0..4 {
                debug_assert!(target_index * 4 + byte < end * 4);
                target_data.set(target_index * 4 + byte, 0);
            }
        }
        self.samples.set(Some(target_data.len() / 4));
        target_audio.set_data(target_data);
    }

    #[method]
    fn clear(&self) {
        let target_audio = unsafe { self.audio.assume_safe() };
        target_audio.set_data(ByteArray::new());
        self.samples.set(Some(target_audio.data().len()));
    }
}

fn pcm_sample_to_float(byte0: u8, byte1: u8) -> f32 {
    let byte1: i8 = byte1.wrapping_cast();
    byte1 as f32 / 128.0 + byte0 as f32 / (65536.0 / 2.0)
}

fn init(handle: InitHandle) {
    handle.add_class::<MusicClip>();
}

godot_init!(init);
