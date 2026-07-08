//! Icon of a service in the side rail.
//!
//! A neutral rounded "tile" (same color for all, derived from the theme's text
//! color, so it adapts to light/dark) with the favicon centered on top — the
//! colored favicons stand out on their own, for a cohesive and discreet look.
//! While the favicon has not loaded, it shows the name's initial. An unread
//! badge appears in the top-right corner.

use gtk::cairo;
use gtk::gdk;
use gtk::prelude::*;

const TILE: i32 = 40;
const RADIUS: f64 = 11.0;
/// Opacity of the neutral tile (over the theme's text color).
const TILE_ALPHA: f64 = 0.10;
const BADGE_MAX: u32 = 99;

#[derive(Clone)]
pub struct ServiceIcon {
    root: gtk::Overlay,
    image: gtk::Image,
    label: gtk::Label,
    badge: gtk::Label,
}

impl ServiceIcon {
    pub fn new(name: &str) -> Self {
        let background = build_tile_background();
        let label = build_initial_label(name);
        let image = build_favicon_image();
        let badge = build_unread_badge();

        let root = gtk::Overlay::new();
        root.set_size_request(TILE, TILE);
        root.set_child(Some(&background));
        root.add_overlay(&label);
        root.add_overlay(&image);
        root.add_overlay(&badge);

        Self {
            root,
            image,
            label,
            badge,
        }
    }

    /// Sets the unread count shown in the badge (0 hides it).
    pub fn set_badge(&self, count: u32) {
        if count == 0 {
            self.badge.set_visible(false);
            return;
        }
        let text = if count > BADGE_MAX {
            format!("{BADGE_MAX}+")
        } else {
            count.to_string()
        };
        self.badge.set_label(&text);
        self.badge.set_visible(true);
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    /// Updates the icon with the favicon (or falls back to the initial if `None`).
    pub fn set_favicon(&self, texture: Option<&gdk::Texture>) {
        match texture {
            Some(tex) => {
                let native = tex.width().max(tex.height());
                // Lean size: a smaller favicon, with some breathing room in the tile.
                let size = native.clamp(16, 24);
                self.image.set_pixel_size(size);
                self.image.set_paintable(Some(tex));
                self.image.set_visible(true);
                self.label.set_visible(false);
            }
            None => {
                self.image.set_paintable(None::<&gdk::Texture>);
                self.image.set_visible(false);
                self.label.set_visible(true);
            }
        }
    }
}

fn build_tile_background() -> gtk::DrawingArea {
    let background = gtk::DrawingArea::new();
    background.set_content_width(TILE);
    background.set_content_height(TILE);
    background.set_draw_func(|area, cr, w, h| {
        let c = area.color();
        rounded_rect(cr, 0.5, 0.5, w as f64 - 1.0, h as f64 - 1.0, RADIUS);
        cr.set_source_rgba(c.red() as f64, c.green() as f64, c.blue() as f64, TILE_ALPHA);
        let _ = cr.fill();
    });
    background
}

/// Placeholder shown until the favicon loads: the name's uppercase initial.
fn build_initial_label(name: &str) -> gtk::Label {
    let initial = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    let label = gtk::Label::new(Some(&initial));
    label.add_css_class("service-initial");
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::Center);
    label
}

fn build_favicon_image() -> gtk::Image {
    let image = gtk::Image::new();
    image.set_halign(gtk::Align::Center);
    image.set_valign(gtk::Align::Center);
    image.set_visible(false);
    image
}

fn build_unread_badge() -> gtk::Label {
    let badge = gtk::Label::new(None);
    badge.add_css_class("unread-badge");
    badge.set_halign(gtk::Align::End);
    badge.set_valign(gtk::Align::Start);
    badge.set_visible(false);
    badge
}

/// Traces a rounded rectangle onto the Cairo context (path only, no fill).
fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let d = std::f64::consts::PI / 180.0;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -90.0 * d, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, 90.0 * d);
    cr.arc(x + r, y + h - r, r, 90.0 * d, 180.0 * d);
    cr.arc(x + r, y + r, r, 180.0 * d, 270.0 * d);
    cr.close_path();
}
