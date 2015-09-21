#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    Close,
    MouseDown(u32, u32),
    MouseUp(u32, u32),
    KeyDown(u32),
    KeyUp(u32),
    Size(u32, u32),
}
