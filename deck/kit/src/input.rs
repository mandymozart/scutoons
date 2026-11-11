//! What a person does to the kit's controls (#386): one input for every platform – a key, a gamepad's button, the
//! pointer, a finger. An adapter turns its own events into these (winit's keys and mouse, gilrs's pad, the DOM's keys
//! and pointer events) and hands them to whoever owns the controls (deck-ui `menu.rs`); what an input means there is
//! that owner's. Pointer places are device pixels of the screen the controls were painted for.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    /// take what the keys are on (Enter, a pad's A)
    Enter,
    /// one step back (Esc, Backspace, a pad's B)
    Back,
    /// open the menu (Esc on the native deck with nothing open, M in the browser, a pad's Start, the status line)
    Open,
    /// the pointer moved to (x, y): hovering chooses
    Point(f32, f32),
    /// a press at (x, y) – a click, a tap
    Press(f32, f32),
    /// the secondary press (a right click): back
    Secondary,
}
