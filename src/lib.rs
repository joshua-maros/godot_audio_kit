use std::f32::consts::TAU;

use gdnative::{
    api::{
        AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, AudioStreamPlayer3D,
        AudioStreamSample,
    },
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

enum AudioData {
    Static(Ref<AudioStreamSample>),
    Dynamic(Vector2Array),
}

#[derive(NativeClass)]
#[inherit(Reference)]
pub struct AudioContainer {
    data: AudioData,
}

#[methods]
impl AudioContainer {
    fn new(_base: &Reference) -> Self {
        AudioContainer {
            data: AudioData::Dynamic(Vector2Array::from_iter(std::iter::empty())),
        }
    }

    #[method]
    fn test(&mut self, #[base] _base: &Reference) {
        self.data = AudioData::Dynamic(Vector2Array::from_iter(
            (0..44100)
                .into_iter()
                .map(|x| (x as f32 * 440.0 * TAU / 44100.0).sin())
                .map(|sample| Vector2::new(sample, sample)),
        ));
    }

    #[method]
    fn load_sample_data(&mut self, #[base] _base: &Reference, from: Ref<AudioStreamSample>) {
        self.data = AudioData::Static(from)
    }

    #[method]
    fn play_audio(&self, #[base] _base: &Reference, output: Ref<AudioStreamPlayer3D>) {
        let output = unsafe { output.assume_safe() };
        match &self.data {
            AudioData::Static(sample) => output.set_stream(sample),
            AudioData::Dynamic(data) => {
                let sample = AudioStreamGenerator::new();
                sample.set_buffer_length(data.len() as f64 / 44100.0);
                output.set_stream(sample);
                let playback = output.get_stream_playback().unwrap();
                let playback = playback.try_cast::<AudioStreamGeneratorPlayback>().unwrap();
                let playback = unsafe { playback.assume_safe() };
                playback.push_buffer(data.clone());
            }
        }
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<HelloWorld>();
    handle.add_class::<AudioContainer>();
}

godot_init!(init);
