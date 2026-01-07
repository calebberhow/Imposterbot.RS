macro_rules! const_color {
    ($name:ident, $value:expr) => {
        paste::paste! {
            pub const $name: u32 = $value;

            pub fn [<$name:lower>]() -> poise::serenity_prelude::Colour {
                poise::serenity_prelude::Colour::new($name)
            }
        }
    };
    ($name:ident, $r:expr, $g:expr, $b:expr) => {
        paste::paste! {
            pub const $name: u32 = ($r as u32) << 16 | ($g as u32) << 8 | ($b as u32);

            pub fn [<$name:lower>]() -> poise::serenity_prelude::Colour {
                poise::serenity_prelude::Colour::new($name)
            }
        }
    };
}

const_color! { ORANGE,      0xFF6347 }
const_color! { PURPLE,      0x7D39EE }
const_color! { GREEN,       0x10611B }
const_color! { SLATE,       0x3E6775 }
const_color! { ROYAL_BLUE,  0x000082 }
const_color! { RED,         0xA20000 }
const_color! { LIME,        0x00C100 }

const_color! { BLACK, 0, 0, 0}
const_color! { WHITE, 255, 255, 255}
