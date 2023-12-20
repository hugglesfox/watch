use crate::lcd::segment::*;
use paste::paste;

macro_rules! digits {
    ($($name:ident => $segs:expr),*) => {
        $(
            pub const $name: Segments = $segs;
        )*
    };
}

macro_rules! segments {
    ($digit:expr; $($seg:ident),*) => {
        paste! {
            {
                let mut res = 0;
                $(
                    res |= [<D $digit _ $seg>];
                )*
                res
            }
        }
    };
}

pub const fn digit(digit: u32, seg: u32) -> Segments {
    match digit {
        0 => segments!(digit; A, B, C, D, E, F),
        1 => segments!()
    } 
}

digits! {
    D0_0 => D0_AD | D0_B | D0_C | D0_E | D0_F,
    D0_1 => D0_B | D0_C,
    D0_2 => D0_AD | D0_B | D0_E | D0_G,
    D0_3 => D0_AD | D0_B | D0_C | D0_G,
    D0_4 => D0_F | D0_B | D0_G | D0_C,
    D0_5 => D0_AD | D0_F | D0_G | D0_C,

    D1_0 => D1_A | D1_B | D1_C | D1_D | D1_E | D1_F,

}