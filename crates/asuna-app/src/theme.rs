//! AsuNa's own palette, drawn on top of gpui-component's dark theme.

use gpui_kit::{rgb, rgba, Rgba};

pub const BG: u32 = 0x0b0c10;
pub const PANEL: u32 = 0x101218;
pub const PANEL_2: u32 = 0x171a22;
pub const HOVER: u32 = 0x1f232d;
pub const LINE: u32 = 0x1d2029;
pub const TEXT: u32 = 0xf1f2f5;
pub const MUTED: u32 = 0x8d93a3;
pub const SUBTLE: u32 = 0x5d6373;
pub const ACCENT: u32 = 0xff6b8b;
pub const ACCENT_2: u32 = 0xff9a6b;
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
