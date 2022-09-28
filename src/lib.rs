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
pub struct MusicSample {
    #[property]
    audio: Option<Ref<AudioStreamSample>>,
    #[property]
    samples_per_beat: i32,
    #[property]
    first_beat_sample: i32,
}

#[methods]
impl MusicSample {
    pub fn new(_base: &Resource) -> Self {
        Self {
            audio: None,
            samples_per_beat: 22050,
            first_beat_sample: 0,
        }
    }
}

enum MusicBufferContent {
    Static(Instance<MusicSample>),
    Dynamic {
        audio: Vector2Array,
        samples_per_beat: i32,
        first_beat_sample: i32,
    },
}

#[derive(NativeClass)]
#[inherit(Reference)]
pub struct MusicBuffer {
    content: MusicBufferContent,
}

#[methods]
impl MusicBuffer {
    fn new(_base: &Reference) -> Self {
        MusicBuffer {
            content: MusicBufferContent::Dynamic {
                audio: Vector2Array::from_iter(std::iter::empty()),
                samples_per_beat: 22050,
                first_beat_sample: 0,
            },
        }
    }

    #[method]
    fn load_sample_data(&mut self, from: Instance<MusicSample>) {
        self.content = MusicBufferContent::Static(from)
    }

    #[method]
    fn samples_per_beat(&self) -> i32 {
        match &self.content {
            MusicBufferContent::Static(sample) => {
                let sample = unsafe { sample.assume_safe() };
                let value = sample.map(|sample, _base| sample.samples_per_beat);
                value.unwrap()
            }
            MusicBufferContent::Dynamic {
                samples_per_beat, ..
            } => *samples_per_beat,
        }
    }

    #[method]
    fn first_beat_sample(&self) -> i32 {
        match &self.content {
            MusicBufferContent::Static(sample) => {
                let sample = unsafe { sample.assume_safe() };
                let value = sample.map(|sample, _base| sample.first_beat_sample);
                value.unwrap()
            }
            MusicBufferContent::Dynamic {
                first_beat_sample, ..
            } => *first_beat_sample,
        }
    }

    #[method]
    fn num_samples(&self) -> i32 {
        match &self.content {
            MusicBufferContent::Static(sample) => {
                let sample = unsafe { sample.assume_safe() };
                let sample = sample.map(|sample, _base| sample.audio.clone());
                let sample = sample.unwrap().unwrap();
                let sample = unsafe { sample.assume_safe() };
                sample.data().len() / 4
            }
            MusicBufferContent::Dynamic { audio, .. } => audio.len(),
        }
    }

    #[method]
    fn beat_time(&self) -> f64 {
        self.samples_per_beat() as f64 / 44100.0
    }

    #[method]
    fn start_time(&self) -> f64 {
        self.first_beat_sample() as f64 / 44100.0
    }

    #[method]
    fn duration(&self) -> f64 {
        self.num_samples() as f64 / 44100.0
    }

    #[method]
    fn play_audio(&self, output: Ref<AudioStreamPlayer3D>) {
        let output = unsafe { output.assume_safe() };
        match &self.content {
            MusicBufferContent::Static(sample) => {
                let sample = unsafe { sample.assume_safe() };
                let sample = sample.map(|sample, _base| sample.audio.clone());
                output.set_stream(sample.unwrap().unwrap())
            }
            MusicBufferContent::Dynamic { audio, .. } => {
                let sample = AudioStreamGenerator::new();
                sample.set_buffer_length(audio.len() as f64 / 44100.0);
                output.set_stream(sample);
                let playback = output.get_stream_playback().unwrap();
                let playback = playback.try_cast::<AudioStreamGeneratorPlayback>().unwrap();
                let playback = unsafe { playback.assume_safe() };
                playback.push_buffer(audio.clone());
            }
        }
    }

    #[method]
    fn trim(&self, start: f64, end: f64) -> Instance<Self, Unique> {
        let start = start - self.start_time();
        let end = end - self.start_time();
        let start = (self.samples_per_beat() as f64 * start) as i32 + self.first_beat_sample();
        let end = (self.samples_per_beat() as f64 * end) as i32 + self.first_beat_sample();
        let mut result = Vector2Array::new();
        match &self.content {
            MusicBufferContent::Static(sample) => {
                let sample = unsafe { sample.assume_safe() };
                let sample = sample.map(|sample, _base| sample.audio.clone());
                let sample = sample.unwrap().unwrap();
                let sample = unsafe { sample.assume_safe() };
                let pcm_data = sample.data();
                for i in start..end {
                    let byte_index = i * 4;
                    assert!(byte_index < pcm_data.len());
                    let left = pcm_sample_to_float(
                        pcm_data.get(byte_index + 0),
                        pcm_data.get(byte_index + 1),
                    );
                    let right = pcm_sample_to_float(
                        pcm_data.get(byte_index + 2),
                        pcm_data.get(byte_index + 3),
                    );
                    result.push(Vector2::new(left, right));
                }
            }
            MusicBufferContent::Dynamic { audio, .. } => {
                for i in start..end {
                    result.push(audio.get(i));
                }
            }
        }
        Instance::emplace(Self {
            content: MusicBufferContent::Dynamic {
                audio: Vector2Array::new(),
                samples_per_beat: self.samples_per_beat(),
                first_beat_sample: self.first_beat_sample(),
            },
        })
    }
}

fn pcm_sample_to_float(byte0: u8, byte1: u8) -> f32 {
    let byte1: i8 = byte1.wrapping_cast();
    byte1 as f32 / 128.0 + byte0 as f32 / (65536.0 / 2.0)
}

#[derive(NativeClass)]
#[inherit(Reference)]
struct Dummy(Vector2Array);

#[methods]
impl Dummy {
    fn new(_base: &Reference) -> Self {
        Self(Vector2Array::from_iter(std::iter::empty()))
    }

    #[method]
    fn make(&self) -> Instance<Self, Unique> {
        println!("{:#?}", self.0);
        Instance::emplace(Self(Vector2Array::from_iter(std::iter::empty())))
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<MusicSample>();
    handle.add_class::<MusicBuffer>();
    handle.add_class::<Dummy>();
}

godot_init!(init);
