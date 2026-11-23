//! Horizon's own mark as the window draws it: the game folder's row on the rail, the mark over
//! the rail in the top bar, and Home's title. Decoded once per size and kept in `Launcher::marks`.
use super::*;
use gpui::RenderImage;
use std::sync::Arc;

/// Horizon's mark (`docs/media/brand/icon.svg`), as the icon theme holds it.
const MARK: &[u8] = include_bytes!("../../packaging/icons/hicolor/256x256/apps/horizon.png");

impl Launcher {
    /// Horizon's own mark, `size` points square: the brand icon, its tile already rounded.
    pub(super) fn horizon_mark(&mut self, size: f32) -> AnyElement {
        let key = (size * self.scale) as u32;
        let image = self
            .marks
            .entry(key)
            .or_insert_with(|| fitted(MARK.to_vec(), size, self.scale))
            .clone();
        match image {
            Some(image) => centred(image, size, self.scale).into_any_element(),
            None => div().size(px(size)).flex_none().into_any_element(),
        }
    }
}

/// An image file's bytes drawn at most `size` points square: scaled down smoothly when larger,
/// up by whole device pixels when smaller, so pixel-art logos stay sharp.
fn fitted(bytes: Vec<u8>, size: f32, scale: f32) -> Option<Arc<RenderImage>> {
    let side = (size * scale).round().max(1.) as u32;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 << 20);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let scaled = if decoded.width() > side || decoded.height() > side {
        decoded.resize(side, side, image::imageops::FilterType::Triangle)
    } else {
        decoded
    };
    let image = scaled.to_rgba8();
    let (w, h) = (image.width() as usize, image.height() as usize);
    let texels: Vec<[u8; 4]> = image.pixels().map(|p| p.0).collect();
    let whole = (side as usize / w.max(h)).max(1) as f32;
    Some(art::bake(&texels, w, h, whole / scale, scale))
}

/// An image made by [`fitted`], centred in a `size` points square.
fn centred(image: Arc<RenderImage>, size: f32, scale: f32) -> Div {
    let dim = image.size(0);
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .child(
            gpui::img(image)
                .w(px(dim.width.0 as f32 / scale))
                .h(px(dim.height.0 as f32 / scale)),
        )
}
