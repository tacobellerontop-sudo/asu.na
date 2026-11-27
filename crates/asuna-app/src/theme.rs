//! AsuNa's own palette, drawn on top of gpui-component's dark theme.

use gpui_kit::{rgb, rgba, Rgba};

pub const BG: u32 = 0x0e0f13;
pub const PANEL: u32 = 0x15171d;
pub const PANEL_2: u32 = 0x1c1f27;
pub const HOVER: u32 = 0x232733;
pub const LINE: u32 = 0x262a34;
pub const TEXT: u32 = 0xe9ebf0;
pub const MUTED: u32 = 0x8b91a0;
pub const ACCENT: u32 = 0xff6b8b;
pub const ACCENT_2: u32 = 0xff8f6b;
pub const OK: u32 = 0x3ddc97;
pub const WARN: u32 = 0xf5a524;
pub const BAD: u32 = 0xff5c5c;
pub const FAV: u32 = 0xffd24d;

pub fn c(hex: u32) -> Rgba {
    rgb(hex)
}

/// Color with alpha (0-255).
pub fn ca(hex: u32, alpha: u8) -> Rgba {
    rgba((hex << 8) | alpha as u32)
}
