//! Light theme loosely following the Lark desktop look: neutral greys, a
//! light navigation rail and the #3370FF accent. Colours only, no assets.

pub const ACCENT: u32 = 0x3370ff;
pub const ACCENT_SOFT: u32 = 0xe1eaff;
pub const ACCENT_HOVER: u32 = 0x245bdb;
pub const RAIL_BG: u32 = 0xeff0f1;
pub const PANEL_BG: u32 = 0xf7f8fa;
pub const MAIN_BG: u32 = 0xffffff;
pub const HOVER: u32 = 0xeff0f1;
pub const BORDER: u32 = 0xdee0e3;
pub const TEXT: u32 = 0x1f2329;
pub const TEXT_2: u32 = 0x646a73;
pub const MUTED: u32 = 0x8f959e;
pub const BUBBLE_OTHER: u32 = 0xf2f3f5;
pub const ERROR: u32 = 0xf54a45;
pub const SUCCESS: u32 = 0x34c724;
pub const WHITE: u32 = 0xffffff;
pub const CODE_BG: u32 = 0x1f2329;
pub const CODE_TEXT: u32 = 0xe8eaed;

/// Deterministic avatar colour for a name.
pub fn avatar_color(seed: &str) -> u32 {
    const PALETTE: [u32; 6] = [0x3370ff, 0x7f3bf5, 0x00b5a8, 0xff8800, 0xf54a45, 0x2ea121];
    let h = seed.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[(h as usize) % PALETTE.len()]
}
