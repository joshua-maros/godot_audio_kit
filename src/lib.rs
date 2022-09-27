use gdnative::prelude::*;

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
#[inherit(Reference)]
pub struct AudioSource;

#[methods]
impl AudioSource {
    fn new(_base: &Reference) -> Self {
        AudioSource
    }

    #[method]
    fn test(&self, #[base] _base: &Reference) {
        
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<HelloWorld>();
}

godot_init!(init);
