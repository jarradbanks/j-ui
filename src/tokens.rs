//! Soft component tokens. Sizes are egui points; the profile is documented in `DESIGN.md`.

use egui::Color32;

pub(crate) const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Colours. The themed ones are functions returning the applied theme's value
/// ([`crate::themes::apply`]); [`crate::themes::DARK`] holds the `DESIGN.md` starting values.
pub mod color {
    use super::hex;
    use egui::Color32;

    /// The original Dark hover colour. Use [`blue_hover`] for the applied theme.
    pub const BLUE_HOVER: Color32 = crate::themes::DARK.blue_hover;
    pub const BLUE_PRESSED: Color32 = hex(0x215DB3);

    pub const CLIP_VIOLET: Color32 = hex(0x5E589F);
    pub const CLIP_PLUM: Color32 = hex(0x8E4771);
    pub const CLIP_OCHRE: Color32 = hex(0x88602D);
    pub const CLIP_GREEN: Color32 = hex(0x30715D);
    pub const CLIPS: [Color32; 4] = [CLIP_VIOLET, CLIP_PLUM, CLIP_OCHRE, CLIP_GREEN];

    pub const VIEWER: Color32 = hex(0x040404);
    pub const WHITE: Color32 = Color32::WHITE;

    /// Soft control and selection fill, with readable text in high-contrast palettes.
    pub fn selection_fill() -> Color32 {
        let palette = crate::themes::current();
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line
            && super::contrast(palette.panel, Color32::BLACK) > 10.0
        {
            return palette.raised;
        }
        palette.selection_fill()
    }

    /// Border-toned surfaces, with an accessible fill in High Contrast.
    pub fn neutral_fill() -> Color32 {
        crate::themes::current().selection_fill()
    }

    /// Separates surfaces and controls when the palette needs contrast outlines.
    pub fn surface_outline() -> Color32 {
        if crate::themes::current().selection_fill() != border() {
            return border();
        }
        Color32::TRANSPARENT
    }

    /// Recessed field fill for the current presentation.
    pub fn field_fill() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Soft {
            ground()
        } else {
            field()
        }
    }

    /// Resting secondary button fill.
    pub fn button_fill() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Soft {
            selection_fill()
        } else {
            raised()
        }
    }

    /// Control edge; Soft relies on tone steps except in High Contrast.
    pub fn control_outline() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Soft {
            surface_outline()
        } else {
            border()
        }
    }

    /// Panel surface fill; Line leaves the document surface visible.
    pub fn panel_fill() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line {
            Color32::TRANSPARENT
        } else {
            panel()
        }
    }

    /// Panel edge; Current and High Contrast retain a visible stroke.
    pub fn panel_outline() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Current {
            border()
        } else {
            surface_outline()
        }
    }

    /// Card surface fill in the active presentation.
    pub fn card_fill() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line {
            panel()
        } else {
            raised()
        }
    }

    /// Background behind the segmented choices.
    pub fn segment_track() -> Color32 {
        match crate::theme::component_style() {
            crate::theme::ComponentStyle::Current => field(),
            crate::theme::ComponentStyle::Line => Color32::TRANSPARENT,
            crate::theme::ComponentStyle::Soft => ground(),
        }
    }

    /// Selected segment fill in the active presentation.
    pub fn segment_fill() -> Color32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Current
            || surface_outline() != Color32::TRANSPARENT
        {
            selection_fill()
        } else {
            raised()
        }
    }

    macro_rules! themed {
        ($($name:ident),*) => {$(
            pub fn $name() -> Color32 {
                crate::themes::current().$name
            }
        )*};
    }
    themed!(
        blue,
        blue_soft,
        blue_hover,
        light_blue,
        red,
        red_soft,
        amber,
        amber_soft,
        green,
        green_soft,
        ground,
        panel,
        raised,
        field,
        border,
        border_strong,
        ink,
        ink_2,
        ink_3,
        ink_4
    );
}

/// Spacing scale (4px base).
pub mod space {
    pub const S4: f32 = 4.0;
    pub const S8: f32 = 8.0;
    pub const S12: f32 = 12.0;
    pub const S16: f32 = 16.0;
    pub const S24: f32 = 24.0;
    pub const S32: f32 = 32.0;
    /// Padding inside a panel and its header.
    pub const INSET: f32 = S16;
    /// Between panels, and between panels and the window edge.
    pub const GUTTER: f32 = S16;
    /// Between controls in a row or a stack.
    pub const GAP: f32 = S12;
    /// Left and right padding inside a button.
    pub const BUTTON_PAD: f32 = S16;
    /// Left and right padding inside a field or a menu row.
    pub const FIELD_PAD: f32 = S12;
    /// Padding inside a dialog.
    pub const DIALOG_PAD: f32 = S24;
    /// `INSET` in the active component presentation.
    pub fn inset() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 12.0,
            _ => INSET,
        }
    }

    /// `GAP` in the active component presentation.
    pub fn gap() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 8.0,
            _ => GAP,
        }
    }

    /// `BUTTON_PAD` in the active component presentation.
    pub fn button_pad() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 10.0,
            _ => BUTTON_PAD,
        }
    }

    /// `FIELD_PAD` in the active component presentation.
    pub fn field_pad() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 8.0,
            _ => FIELD_PAD,
        }
    }

    /// `DIALOG_PAD` in the active component presentation.
    pub fn dialog_pad() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 16.0,
            _ => DIALOG_PAD,
        }
    }

    /// Panel padding in points; Line has unboxed sections.
    pub fn panel_inset() -> f32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line {
            0.0
        } else {
            inset()
        }
    }
    /// Space between document sections in points.
    pub fn section_gap() -> f32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line {
            40.0
        } else {
            S16
        }
    }
}

/// Corner radii. Every corner in the suite comes from this scale.
pub mod radius {
    pub const SMALL: u8 = 8;
    pub const CONTROL: u8 = 10;
    pub const PANEL: u8 = 16;
    pub const DIALOG: u8 = 20;
    pub const SCALE: [u8; 4] = [SMALL, CONTROL, PANEL, DIALOG];
    /// `SMALL` in the active component presentation.
    pub fn small() -> u8 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 4,
            ComponentStyle::Line => 6,
            _ => SMALL,
        }
    }

    /// `CONTROL` in the active component presentation.
    pub fn control() -> u8 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 6,
            ComponentStyle::Line => 8,
            _ => CONTROL,
        }
    }

    /// `PANEL` in the active component presentation.
    pub fn panel() -> u8 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 8,
            ComponentStyle::Line => 12,
            _ => PANEL,
        }
    }

    /// `DIALOG` in the active component presentation.
    pub fn dialog() -> u8 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 12,
            ComponentStyle::Line => 16,
            _ => DIALOG,
        }
    }
}

/// Fixed component sizes.
pub mod size {
    pub const APP_BAR: f32 = 56.0;
    pub const HEADER: f32 = 48.0;
    pub const RULER: f32 = 24.0;
    pub const CONTROL: f32 = 36.0;
    pub const TAB_MAX: f32 = 160.0;
    pub const TAB_MIN: f32 = 64.0;
    pub const SMALL_CONTROL: f32 = 28.0;
    pub const ICON: f32 = 18.0;
    pub const BADGE: f32 = 24.0;
    /// Thickness of a slider or progress track.
    pub const SLIDER_TRACK: f32 = 6.0;
    /// Diameter of a slider thumb.
    pub const THUMB: f32 = 16.0;
    pub const TRACK_HEADER: f32 = 160.0;
    pub const TRACK: f32 = 40.0;
    pub const INSPECTOR_LABEL: f32 = 72.0;
    pub const PANEL_MIN: f32 = 240.0;
    /// `APP_BAR` in the active component presentation.
    pub fn app_bar() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 40.0,
            _ => APP_BAR,
        }
    }

    /// `HEADER` in the active component presentation.
    pub fn header() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 32.0,
            _ => HEADER,
        }
    }

    /// `CONTROL` in the active component presentation.
    pub fn control() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 24.0,
            _ => CONTROL,
        }
    }

    /// `ICON` in the active component presentation.
    pub fn icon() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 16.0,
            _ => ICON,
        }
    }

    /// `BADGE` in the active component presentation.
    pub fn badge() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 20.0,
            _ => BADGE,
        }
    }

    /// `SLIDER_TRACK` in the active component presentation.
    pub fn slider_track() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 4.0,
            _ => SLIDER_TRACK,
        }
    }

    /// `THUMB` in the active component presentation.
    pub fn thumb() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 12.0,
            _ => THUMB,
        }
    }

    /// Panel title row height in points.
    pub fn panel_header() -> f32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Line {
            28.0
        } else {
            header()
        }
    }
    /// Checkbox/radio diameter in points.
    pub fn selection() -> f32 {
        if crate::theme::component_style() == crate::theme::ComponentStyle::Current {
            14.0
        } else {
            18.0
        }
    }
}

/// Type scale: font sizes in points.
pub mod font {
    pub const DISPLAY: f32 = 28.0;
    pub const HEADLINE: f32 = 20.0;
    pub const TITLE: f32 = 16.0;
    pub const BODY: f32 = 14.0;
    pub const SMALL: f32 = 12.0;
    pub const TIMECODE: f32 = 16.0;
    /// `DISPLAY` in the active component presentation.
    pub fn display() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 22.0,
            _ => DISPLAY,
        }
    }

    /// `HEADLINE` in the active component presentation.
    pub fn headline() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 16.0,
            _ => HEADLINE,
        }
    }

    /// `TITLE` in the active component presentation.
    pub fn title() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 14.0,
            _ => TITLE,
        }
    }

    /// `BODY` in the active component presentation.
    pub fn body() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 12.0,
            _ => BODY,
        }
    }

    /// `SMALL` in the active component presentation.
    pub fn small() -> f32 {
        use crate::theme::ComponentStyle;
        match crate::theme::component_style() {
            ComponentStyle::Current => 10.0,
            _ => SMALL,
        }
    }
}

/// WCAG 2 contrast ratio between two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::DARK;

    #[test]
    fn ink_on_every_clip_colour_reaches_4_5() {
        for clip in color::CLIPS {
            let ratio = contrast(DARK.ink, clip);
            assert!(ratio >= 4.5, "Ink on {clip:?} is {ratio:.2}:1");
        }
    }

    #[test]
    fn documented_text_contrasts_hold() {
        assert!(contrast(color::WHITE, DARK.blue) >= 4.5);
        assert!(contrast(DARK.ink_3, DARK.panel) >= 4.5);
        assert!(contrast(DARK.light_blue, DARK.panel) >= 4.5);
    }
}
