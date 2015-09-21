
const MISSING_CHARACTER_PIECES_START: u32 = 1;
const MISSING_CHARACTER_PIECES_COUNT: u32 = 16*8;
const SYMBOLS_1: u32 = MISSING_CHARACTER_PIECES_START + MISSING_CHARACTER_PIECES_COUNT;
const DIGITS_START: u32 = SYMBOLS_1 + 10; 

pub fn glyph_to_parts(glyph: u32) -> Vec<u32> {
    let simple = match glyph {
        0 => SYMBOLS_1,
        1 => SYMBOLS_1 + 1,
        2 => SYMBOLS_1 + 2,
        3 => SYMBOLS_1 + 3,
        4 => SYMBOLS_1 + 4,
        5 => SYMBOLS_1 + 5,
        6 => SYMBOLS_1 + 6,
        7 => SYMBOLS_1 + 7,
        8 => SYMBOLS_1 + 8,
        9 => SYMBOLS_1 + 9,

        10 => DIGITS_START,
        11 => DIGITS_START + 1,
        12 => DIGITS_START + 2,
        13 => DIGITS_START + 3,
        14 => DIGITS_START + 4,
        15 => DIGITS_START + 5,
        16 => DIGITS_START + 6,
        17 => DIGITS_START + 7,
        18 => DIGITS_START + 8,
        19 => DIGITS_START + 10,
        _ => {
            return vec![0, 1, 19];
        }
    };

    return vec![simple];

}
