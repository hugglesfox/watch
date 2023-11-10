use crate::lcd::segment::*;

macro_rules! digits {
    ($($name:ident => $segs:expr),*) => {
        $(
            pub const $name: Segments = $segs;
        )*
    };
}

digits! {
    D0_0 => D0_AD | D0_B | D0_C | D0_E | D0_F,
    D0_1 => D0_B | D0_C,
    D0_2 => D0_AD | D0_B | D0_E | D0_G
}