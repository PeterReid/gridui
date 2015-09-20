use unicode_segmentation::UnicodeSegmentation;

pub type GlyphCode = u32;


pub fn from_char(ch: char) -> Option<u32> {
    if ch >= 'a' && ch <='z' {
        return Some(0x1000 + (((ch as u32) - ('a' as u32)) << 4));
    }
    
    if ch >= 'A' && ch <='Z' {
        return Some(0x3000 + (((ch as u32) - ('A' as u32)) << 4));
    }
    
    match ch {
        ' ' => Some(0),
        '_' => Some(1),
        '-' => Some(2),
        '.' => Some(3),
        ',' => Some(4),
        '/' => Some(5),
        '\\' => Some(6),
        ':' => Some(7),
        ';' => Some(8),
        '@' => Some(9),
        
        '0' => Some(10),
        '1' => Some(11),
        '2' => Some(12),
        '3' => Some(13),
        '4' => Some(14),
        '5' => Some(15),
        '6' => Some(16),
        '7' => Some(17),
        '8' => Some(18),
        '9' => Some(19),
        _ => None
    }
}

pub fn as_char(character: GlyphCode) -> Option<char> {
    let lower_a_code = 0x1000;
    let after_lowers = lower_a_code + 26*16;
    
    if lower_a_code <= character && character  < after_lowers && (character & 0x0f)==0 {
        return Some(((('a' as u32) + ((character & 0x0ff0)>>4)) as u8) as char);
    }
    
    let case_mask = 0x00002000;
    let upper_a_code = lower_a_code ^ case_mask;
    let after_uppers = after_lowers ^ case_mask;
    if upper_a_code <= character && character < after_uppers && (character & 0x0f)==0 {
        return Some(((('A' as u32) + ((character & 0x0ff0)>>4)) as u8) as char);
    }
    
    Some(match character {
        0 => ' ',
        1 => '_',
        2 => '-',
        3 => '.',
        4 => ',',
        5 => '/',
        6 => '\\',
        7 => ':',
        8 => ';',
        9 => '@',
        
        10 => '0',
        11 => '1',
        12 => '2',
        13 => '3',
        14 => '4',
        15 => '5',
        16 => '6',
        17 => '7',
        18 => '8',
        19 => '9',
        _ => { return None; },
    })
}

pub fn to_string(glyphcodes: &[u32]) -> Option<String> {
    let mut accum = String::new();
    for glyphcode in glyphcodes.iter() {
        if let Some(ch) = as_char(*glyphcode) {
            accum.push(ch);
        } else  {
            return None;
        }
    }
    return Some(accum);
}

pub fn from_str(s: &str) -> Option<Vec<u32>> {
    let mut accum = Vec::new();
    
    for grapheme in UnicodeSegmentation::graphemes(s, true) {
        if grapheme.len()!=1 {
            return None;
        }
        if let Some(glyphcode) = grapheme.chars().next().and_then(|x| from_char(x)) {
            accum.push(glyphcode);
        } else {
            return None;
        }
    }
    return Some(accum);
}
    

#[cfg(test)]
mod test {
    use super::{from_str, to_string};

    fn test_str(s: &str, expected: &[u32]) {
        assert_eq!(from_str(s), Some(expected.to_vec()));
        assert_eq!(to_string(&from_str(s).unwrap()[]), Some(s.to_string()));
    }

    #[test]
    fn from_strs() {
        test_str("abc", &[0x1000, 0x1010, 0x1020]);
        test_str("Abc", &[0x3000, 0x1010, 0x1020]);
    }
}
