use gpui_kit::component::{Theme, ThemeColor, ThemeMode};
use gpui_kit::{App, Hsla, Rgba, Window};

use crate::palette::{Appearance, AppearancePalette, Srgb};

pub fn to_hsla(color: Srgb) -> Hsla {
    let (red, green, blue, alpha) = color.channel_unit();
    Rgba {
        r: red,
        g: green,
        b: blue,
        a: alpha,
    }
    .into()
}

pub fn apply_appearance(appearance: Appearance, window: Option<&mut Window>, cx: &mut App) {
    let mode = match appearance {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
    };
    Theme::change(mode, None, cx);
    let palette = AppearancePalette::for_appearance(appearance);
    let mut colors = match appearance {
        Appearance::Dark => *ThemeColor::dark(),
        Appearance::Light => *ThemeColor::light(),
    };
    paint_palette(&mut colors, &palette);
    {
        let theme = Theme::global_mut(cx);
        theme.colors = colors;
        theme.tokens = colors.into();
        theme.mode = mode;
    }
    Theme::sync_base(cx);
    if let Some(window) = window {
        window.refresh();
    }
}

fn paint_palette(colors: &mut ThemeColor, palette: &AppearancePalette) {
    let base = to_hsla(palette.bg_base);
    let surface = to_hsla(palette.bg_surface);
    let elevated = to_hsla(palette.bg_elevated);
    let hover = to_hsla(palette.bg_hover);
    let active = to_hsla(palette.bg_active);
    let border = to_hsla(palette.border_default);
    let border_subtle = to_hsla(palette.border_subtle);
    let text = to_hsla(palette.text_primary);
    let text_secondary = to_hsla(palette.text_secondary);
    let text_muted = to_hsla(palette.text_muted);
    let link = to_hsla(palette.text_link);
    let accent = to_hsla(palette.accent);
    let accent_hover = to_hsla(palette.accent_hover);
    let accent_contrast = to_hsla(palette.accent_contrast);
    let success = to_hsla(palette.success);
    let success_contrast = to_hsla(palette.success_contrast);
    let warning = to_hsla(palette.warning);
    let warning_contrast = to_hsla(palette.warning_contrast);
    let danger = to_hsla(palette.danger);
    let danger_contrast = to_hsla(palette.danger_contrast);
    let mention = to_hsla(palette.mention_bg);

    colors.background = base;
    colors.foreground = text;
    colors.border = border;
    colors.window_border = border;
    colors.title_bar = surface;
    colors.title_bar_border = border_subtle;
    colors.status_bar = surface;
    colors.status_bar_border = border_subtle;
    colors.primary = accent;
    colors.primary_hover = accent_hover;
    colors.primary_active = accent_hover;
    colors.primary_foreground = accent_contrast;
    colors.button_primary = accent;
    colors.button_primary_hover = accent_hover;
    colors.button_primary_active = accent_hover;
    colors.button_primary_foreground = accent_contrast;
    colors.secondary = elevated;
    colors.secondary_hover = hover;
    colors.secondary_active = active;
    colors.secondary_foreground = text_secondary;
    colors.button = elevated;
    colors.button_hover = hover;
    colors.button_active = active;
    colors.button_foreground = text;
    colors.muted = hover;
    colors.muted_foreground = text_muted;
    colors.accent = hover;
    colors.accent_foreground = text;
    colors.danger = danger;
    colors.danger_hover = danger;
    colors.danger_active = danger;
    colors.danger_foreground = danger_contrast;
    colors.button_danger = danger;
    colors.button_danger_hover = danger;
    colors.button_danger_active = danger;
    colors.button_danger_foreground = danger_contrast;
    colors.success = success;
    colors.success_hover = success;
    colors.success_active = success;
    colors.success_foreground = success_contrast;
    colors.button_success = success;
    colors.button_success_hover = success;
    colors.button_success_active = success;
    colors.button_success_foreground = success_contrast;
    colors.warning = warning;
    colors.warning_hover = warning;
    colors.warning_active = warning;
    colors.warning_foreground = warning_contrast;
    colors.button_warning = warning;
    colors.button_warning_hover = warning;
    colors.button_warning_active = warning;
    colors.button_warning_foreground = warning_contrast;
    colors.link = link;
    colors.link_hover = link;
    colors.link_active = link;
    colors.ring = accent;
    colors.selection = mention;
    colors.caret = accent;
    colors.popover = elevated;
    colors.popover_foreground = text;
    colors.sidebar = surface;
    colors.sidebar_foreground = text;
    colors.sidebar_border = border_subtle;
    colors.sidebar_accent = hover;
    colors.sidebar_accent_foreground = text;
    colors.sidebar_primary = accent;
    colors.sidebar_primary_foreground = accent_contrast;
    colors.list = base;
    colors.list_even = surface;
    colors.list_hover = hover;
    colors.list_active = active;
    colors.list_active_border = accent;
    colors.list_head = surface;
    colors.tab = surface;
    colors.tab_active = elevated;
    colors.tab_active_foreground = text;
    colors.tab_foreground = text_secondary;
    colors.tab_bar = surface;
    colors.tab_bar_segmented = elevated;
    colors.table = base;
    colors.table_even = surface;
    colors.table_hover = hover;
    colors.table_active = active;
    colors.table_active_border = accent;
    colors.table_head = surface;
    colors.table_head_foreground = text_secondary;
    colors.table_foot = surface;
    colors.table_foot_foreground = text_secondary;
    colors.table_row_border = border_subtle;
    colors.input = border;
    colors.scrollbar = base;
    colors.scrollbar_thumb = border;
    colors.scrollbar_thumb_hover = text_muted;
    colors.skeleton = hover;
    colors.overlay = base;
    colors.tiles = base;
    colors.accordion = surface;
    colors.group_box = elevated;
    colors.group_box_foreground = text;
    colors.progress_bar = accent;
    colors.slider_bar = accent;
    colors.slider_thumb = accent_contrast;
    colors.switch = accent;
    colors.switch_thumb = accent_contrast;
    colors.drop_target = mention;
    colors.drag_border = accent;
    colors.info = link;
    colors.info_hover = link;
    colors.info_active = link;
    colors.info_foreground = accent_contrast;
    colors.green = success;
    colors.green_light = success;
    colors.red = danger;
    colors.red_light = danger;
    colors.yellow = warning;
    colors.yellow_light = warning;
    colors.blue = link;
    colors.blue_light = link;
}
