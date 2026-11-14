//! The light start (#549, research/SHOW.md §15 point 3b): the deck's own UI on a black screen and nothing else – no
//! scene, no passes, no splash, no HUD, no build label, no sound. What a deck that is only there to vote draws with (a
//! phone's voting screen, deck-space audience.rs): the kit's egui screen (`ui.rs`) with its ballot as the layer `VOTE`,
//! by egui-wgpu in one pass on the host's surface – WebGPU and WebGL2 alike. Its only pipeline is egui-wgpu's: nothing
//! of the stage's is compiled, so the first picture comes as soon as the GPU is there.
use super::ui::Ui;
use deck_kit::base::input::Input;
use deck_kit::base::screen::Layer;

pub struct Light {
    ui: Ui,
    size: (u32, u32),
}

impl Light {
    /// `format`: the surface's (a non-sRGB one, as the stage's); `size`: its pixels
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) -> Light {
        Light { ui: Ui::new(device, format), size }
    }

    /// The ballot, or what shows between votes: Some = its layer (deck-ui `ballot::Page::paint`); None = gone. Called when
    /// it changes.
    pub fn set_vote(&mut self, vote: Option<Layer>) {
        self.ui.set_vote(vote);
    }

    /// an input of the person (the kit's, in device pixels) for egui's input state
    pub fn ui_input(&mut self, i: Input) {
        self.ui.input(i);
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.size = size;
    }

    /// One frame into `enc`, on the surface's `view`: black, the UI over it.
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.ui.prepare(device, queue, enc, self.size);
        // egui-wgpu draws into a pass that does not borrow the encoder's lifetime
        let mut rp = super::begin(enc, view).forget_lifetime();
        self.ui.draw(&mut rp);
    }
}
