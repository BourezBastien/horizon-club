//! The blocky look's parts (`docs/design-system.md`, "Blocks"): pixel type, pixel icons, panels
//! with a dark outline, a light bevel on top and a darker lip below, and sunk wells.
use crate::theme::{self, Theme};
use gpui::{div, prelude::*, px, svg, AnyElement, Div, FontWeight, Hsla, SharedString, Svg};
use std::borrow::Cow;

/// The icons, served to `svg()` as `icons/<name>.svg`.
pub struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path
            .strip_prefix("icons/glass/")
            .and_then(|p| p.strip_suffix(".svg"))
        {
            return Ok(horizon_theme::glass_icons::get(name).map(Cow::Borrowed));
        }
        let name = path
            .strip_prefix("icons/")
            .and_then(|p| p.strip_suffix(".svg"));
        Ok(name.and_then(horizon_theme::icons::get).map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(horizon_theme::icons::ALL
            .iter()
            .map(|(name, _)| format!("icons/{name}.svg").into())
            .chain(
                horizon_theme::glass_icons::ALL
                    .iter()
                    .map(|(name, _)| format!("icons/glass/{name}.svg").into()),
            )
            .collect())
    }
}

/// A pixel icon (`horizon_theme::icons`) in `color`.
pub fn icon(name: &'static str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .flex_none()
        .size(px(size))
        .text_color(color)
}

/// Text in the pixel face: headings, navigation, buttons, badges.
pub fn pixel(size: f32, weight: FontWeight) -> Div {
    div()
        .font_family(theme::PIXEL)
        .font_weight(weight)
        .text_size(px(size))
        .line_height(px((size * 1.2).round()))
}

/// The light edge along the top of a raised block and the darker lip along its bottom; the
/// block must be `relative`.
pub fn edges(top: Hsla, bottom: Hsla, lip: f32) -> [AnyElement; 2] {
    [
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(theme::BEVEL))
            .bg(top)
            .into_any_element(),
        div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(lip))
            .bg(bottom)
            .into_any_element(),
    ]
}

/// A panel: rows, cards, notices, dialogs.
pub fn panel(t: &Theme) -> Div {
    // The glass look: a glass plate, round at 16 (its tokens leave no border or bevel to see).
    if t.glass {
        return crate::glass::plate(horizon_theme::glass::radius::POPOVER);
    }
    div()
        .relative()
        .bg(t.panel)
        .border_2()
        .border_color(t.outline)
        .children(edges(t.bevel, t.lip, theme::BEVEL))
}
