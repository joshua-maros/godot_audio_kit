use std::f32::consts::TAU;

use az::WrappingCast;
use gdnative::{
    api::{
        AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, AudioStreamPlayer3D,
        AudioStreamSample, Resource,
    },
    export::user_data::Map,
    prelude::*,
};

#[derive(NativeClass)]
#[inherit(Node)]
pub struct HelloWorld;

#[methods]
impl HelloWorld {
    fn new(_base: &Node) -> Self {
        HelloWorld
    }

    #[method]
    fn _ready(&self, #[base] _base: &Node) {
        godot_print!("Hello, world.");
    }
}

#[derive(NativeClass)]
#[inherit(Resource)]
pub struct MusicClip {
    #[property]
    audio: Option<Ref<AudioStreamSample>>,
    #[property]
    samples_per_beat: i32,
    #[property]
    first_beat_sample: i32,
}

#[methods]
impl MusicClip {
    pub fn new(_base: &Resource) -> Self {
        Self {
            audio: None,
            samples_per_beat: 22050,
            first_beat_sample: 0,
        }
    }

    #[method]
    fn num_samples(&self) -> i32 {
        if let Some(sample) = &self.audio {
            let sample = unsafe { sample.assume_safe() };
            sample.data().len() / 4
        } else {
            0
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
    fn play_audio(&self, output: Ref<AudioStreamPlayer3D>) {
        let output = unsafe { output.assume_safe() };
        if let Some(sample) = &self.audio {
            output.set_stream(sample)
        }
    }

    #[method]
    fn trim(&self, start: f64, end: f64) -> Instance<Self, Unique> {
        let start_sample = (self.samples_per_beat as f64 * start) as i32 + self.first_beat_sample;
        let end_sample = (self.samples_per_beat as f64 * end) as i32 + self.first_beat_sample;
        let mut data = ByteArray::new();
        if let Some(sample) = &self.audio {
            let sample = unsafe { sample.assume_safe() };
            let pcm_data = sample.data();
            for i in start_sample * 4..end_sample * 4 {
                assert!(i < pcm_data.len());
                data.push(pcm_data.get(i));
            }
        }
        let result = AudioStreamSample::new();
        result.set_format(AudioStreamSample::FORMAT_16_BITS);
        result.set_mix_rate(44100);
        result.set_stereo(true);
        result.set_loop_mode(AudioStreamSample::LOOP_DISABLED);
        result.set_data(data);
        Instance::emplace(Self {
            audio: Some(result.into_shared()),
            samples_per_beat: self.samples_per_beat,
            first_beat_sample: (start_sample - self.first_beat_sample) % self.samples_per_beat,
        })
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
