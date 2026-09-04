#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Srgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Srgb {
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub const fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn channel_unit(self) -> (f32, f32, f32, f32) {
        (
            f32::from(self.red) / 255.0,
            f32::from(self.green) / 255.0,
            f32::from(self.blue) / 255.0,
            f32::from(self.alpha) / 255.0,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppearancePalette {
    pub bg_base: Srgb,
    pub bg_surface: Srgb,
    pub bg_elevated: Srgb,
    pub bg_hover: Srgb,
    pub bg_active: Srgb,
    pub border_subtle: Srgb,
    pub border_default: Srgb,
    pub text_primary: Srgb,
    pub text_secondary: Srgb,
    pub text_muted: Srgb,
    pub text_link: Srgb,
    pub accent: Srgb,
    pub accent_hover: Srgb,
    pub accent_contrast: Srgb,
    pub success: Srgb,
    pub success_contrast: Srgb,
    pub warning: Srgb,
    pub warning_contrast: Srgb,
    pub danger: Srgb,
    pub danger_contrast: Srgb,
    pub mention_bg: Srgb,
    pub mention_border: Srgb,
    pub speaking_ring: Srgb,
}

impl AppearancePalette {
    pub const fn dark() -> Self {
        Self {
            bg_base: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            bg_surface: Srgb::from_rgb(0x13, 0x15, 0x19),
            bg_elevated: Srgb::from_rgb(0x1a, 0x1d, 0x22),
            bg_hover: Srgb::from_rgb(0x1f, 0x23, 0x29),
            bg_active: Srgb::from_rgb(0x26, 0x2b, 0x32),
            border_subtle: Srgb::from_rgb(0x1e, 0x21, 0x26),
            border_default: Srgb::from_rgb(0x28, 0x2c, 0x34),
            text_primary: Srgb::from_rgb(0xe7, 0xe9, 0xee),
            text_secondary: Srgb::from_rgb(0x9a, 0xa1, 0xad),
            text_muted: Srgb::from_rgb(0x8b, 0x92, 0xa0),
            text_link: Srgb::from_rgb(0x5a, 0xa9, 0xff),
            accent: Srgb::from_rgb(0xff, 0x8a, 0x3d),
            accent_hover: Srgb::from_rgb(0xff, 0x9d, 0x5c),
            accent_contrast: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            success: Srgb::from_rgb(0x3e, 0xcf, 0x8e),
            success_contrast: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            warning: Srgb::from_rgb(0xff, 0xbf, 0x5a),
            warning_contrast: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            danger: Srgb::from_rgb(0xf2, 0x54, 0x5b),
            danger_contrast: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            mention_bg: Srgb::from_rgba(0xff, 0x8a, 0x3d, 26),
            mention_border: Srgb::from_rgb(0xff, 0x8a, 0x3d),
            speaking_ring: Srgb::from_rgb(0x3e, 0xcf, 0x8e),
        }
    }

    pub const fn light() -> Self {
        Self {
            bg_base: Srgb::from_rgb(0xf4, 0xf5, 0xf7),
            bg_surface: Srgb::from_rgb(0xff, 0xff, 0xff),
            bg_elevated: Srgb::from_rgb(0xee, 0xf0, 0xf3),
            bg_hover: Srgb::from_rgb(0xe6, 0xe8, 0xec),
            bg_active: Srgb::from_rgb(0xdd, 0xe1, 0xe6),
            border_subtle: Srgb::from_rgb(0xe2, 0xe5, 0xea),
            border_default: Srgb::from_rgb(0xcf, 0xd4, 0xdc),
            text_primary: Srgb::from_rgb(0x1a, 0x1d, 0x24),
            text_secondary: Srgb::from_rgb(0x4b, 0x55, 0x63),
            text_muted: Srgb::from_rgb(0x5c, 0x65, 0x70),
            text_link: Srgb::from_rgb(0x0b, 0x57, 0xd0),
            accent: Srgb::from_rgb(0xff, 0x8a, 0x3d),
            accent_hover: Srgb::from_rgb(0xe0, 0x6b, 0x1a),
            accent_contrast: Srgb::from_rgb(0x0d, 0x0e, 0x11),
            success: Srgb::from_rgb(0x0f, 0x7a, 0x52),
            success_contrast: Srgb::from_rgb(0xff, 0xff, 0xff),
            warning: Srgb::from_rgb(0x8a, 0x5a, 0x00),
            warning_contrast: Srgb::from_rgb(0xff, 0xff, 0xff),
            danger: Srgb::from_rgb(0xc6, 0x28, 0x28),
            danger_contrast: Srgb::from_rgb(0xff, 0xff, 0xff),
            mention_bg: Srgb::from_rgba(0xff, 0x8a, 0x3d, 31),
            mention_border: Srgb::from_rgb(0xc2, 0x4e, 0x00),
            speaking_ring: Srgb::from_rgb(0x0f, 0x7a, 0x52),
        }
    }

    pub const fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Self::dark(),
            Appearance::Light => Self::light(),
        }
    }
}

pub const GUILD_RAIL_WIDTH: f32 = 72.0;
pub const CHANNEL_SIDEBAR_WIDTH: f32 = 240.0;
pub const MEMBER_LIST_WIDTH: f32 = 240.0;

#[cfg(test)]
fn linearize(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
fn relative_luminance(color: Srgb) -> f32 {
    let (red, green, blue, _) = color.channel_unit();
    0.2126 * linearize(red) + 0.7152 * linearize(green) + 0.0722 * linearize(blue)
}

#[cfg(test)]
pub fn contrast_ratio(left: Srgb, right: Srgb) -> f32 {
    let left_luminance = relative_luminance(left);
    let right_luminance = relative_luminance(right);
    let (lighter, darker) = if left_luminance > right_luminance {
        (left_luminance, right_luminance)
    } else {
        (right_luminance, left_luminance)
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::{AppearancePalette, contrast_ratio};

    fn assert_text_aa(foreground: super::Srgb, background: super::Srgb) {
        assert!(
            contrast_ratio(foreground, background) >= 4.5,
            "contrast {0} is below 4.5",
            contrast_ratio(foreground, background)
        );
    }

    fn assert_palette_text_aa(palette: AppearancePalette) {
        for background in [palette.bg_base, palette.bg_surface, palette.bg_elevated] {
            assert_text_aa(palette.text_primary, background);
            assert_text_aa(palette.text_secondary, background);
            assert_text_aa(palette.text_muted, background);
            assert_text_aa(palette.text_link, background);
            assert_text_aa(palette.success, background);
            assert_text_aa(palette.warning, background);
            assert_text_aa(palette.danger, background);
        }
        assert_text_aa(palette.accent_contrast, palette.accent);
        assert_text_aa(palette.success_contrast, palette.success);
        assert_text_aa(palette.warning_contrast, palette.warning);
        assert_text_aa(palette.danger_contrast, palette.danger);
    }

    #[test]
    fn dark_palette_meets_wcag_aa_text() {
        assert_palette_text_aa(AppearancePalette::dark());
    }

    #[test]
    fn light_palette_meets_wcag_aa_text() {
        assert_palette_text_aa(AppearancePalette::light());
    }
}
