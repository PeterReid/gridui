
#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub character: u32,
    pub background: u32,
    pub foreground: u32,
}

#[derive(Clone)]
pub struct Screen {
    pub glyphs: Vec<Glyph>,
    pub width: u32,
}
