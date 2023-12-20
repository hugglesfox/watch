//! https://www.sensorwatch.net/docs/wig/display/
//! TODO: Explain how to construct digits from segments and segment layout

/// The segments are stored in a 96 bit integer, 32 bits for each common line
///
/// # Memory map
///
/// ```txt
///       ----------
/// 0x60 | u32 COM2 |
///      | u32 COM1 |
/// 0x00 | u32 COM0 |
///       ----------
/// ```
pub type Segments = u128;

/// Convert an LCD segment pin number to an MCU LCD segment number
const fn lcd_to_mcu(seg: usize) -> usize {
    match seg {
        0 => 16,
        1 => 9,
        2 => 8,
        3 => 7,
        4 => 17,
        5 => 2,
        6 => 15,
        7 => 14,
        13 => 13,
        17 => 12,
        18 => 11,
        19 => 10,
        20 => 6,
        21 => 5,
        22 => 4,
        23 => 3,
        _ => panic!("Invalid segment number"),
    }
}

/// Create a segment from an LCD common and segment line
const fn build_segment(com: usize, seg: usize) -> Segments {
    1 << (lcd_to_mcu(seg) + (com * 32))
}

macro_rules! segments {
    ($($name:ident => ($com:literal, $seg:literal)),*) => {
        $(
            pub const $name: Segments = build_segment($com, $seg);
        )*
    };
}

/// Turn off all segments
pub const BLANK: Segments = 0;

// 7 segment displays are numbered left (hours) to right (seconds), 0 to 5
segments! {
    D0_A => (1, 5),
    D0_B => (0, 4),
    D0_C => (2, 4),
    D0_D => (1, 5),
    D0_E => (2, 5),
    D0_F => (0, 5),
    D0_G => (1, 4),

    D1_A => (0, 3),
    D1_B => (0, 2),
    D1_C => (1, 2),
    D1_D => (2, 2),
    D1_E => (2, 3),
    D1_F => (1, 6),
    D1_G => (1, 3),

    D2_A => (2, 1),
    D2_B => (0, 0),
    D2_C => (2, 0),
    D2_D => (2, 1),
    D2_E => (1, 1),
    D2_F => (0, 1),
    D2_G => (1, 0),

    D3_A => (0, 22),
    D3_B => (0, 13),
    D3_C => (2, 22),
    D3_D => (2, 23),
    D3_E => (1, 23),
    D3_F => (0, 23),
    D3_G => (1, 22),

    D4_A => (0, 21),
    D4_B => (0, 20),
    D4_C => (2, 19),
    D4_D => (2, 20),
    D4_E => (2, 21),
    D4_F => (1, 21),
    D4_G => (1, 20),

    D5_A => (0, 19),
    D5_B => (0, 18),
    D5_C => (1, 17),
    D5_D => (2, 17),
    D5_E => (2, 18),
    D5_F => (1, 19),
    D5_G => (1, 18)
}
