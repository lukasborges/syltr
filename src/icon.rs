//! Ícone de um serviço no rail lateral.
//!
//! Um "tile" arredondado neutro (mesma cor para todos, derivada da cor de
//! texto do tema, então adapta claro/escuro) com o favicon centralizado por
//! cima — os favicons coloridos se destacam sozinhos, visual coeso e discreto.
//! Enquanto o favicon não carregou, mostra a inicial do nome. Um badge de não
//! lidas aparece no canto superior direito.

use gtk::cairo;
use gtk::gdk;
use gtk::prelude::*;

const TILE: i32 = 40;
const RADIUS: f64 = 11.0;
/// Opacidade do tile neutro (sobre a cor de texto do tema).
const TILE_ALPHA: f64 = 0.10;

#[derive(Clone)]
pub struct ServiceIcon {
    root: gtk::Overlay,
    image: gtk::Image,
    label: gtk::Label,
    badge: gtk::Label,
}

impl ServiceIcon {
    pub fn new(name: &str) -> Self {
        let bg = gtk::DrawingArea::new();
        bg.set_content_width(TILE);
        bg.set_content_height(TILE);
        bg.set_draw_func(|area, cr, w, h| {
            // Tile neutro: cor de texto do tema com baixa opacidade.
            let c = area.color();
            rounded_rect(cr, 0.5, 0.5, w as f64 - 1.0, h as f64 - 1.0, RADIUS);
            cr.set_source_rgba(c.red() as f64, c.green() as f64, c.blue() as f64, TILE_ALPHA);
            let _ = cr.fill();
        });

        let initial = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        let label = gtk::Label::new(Some(&initial));
        label.add_css_class("service-initial");
        label.set_halign(gtk::Align::Center);
        label.set_valign(gtk::Align::Center);

        let image = gtk::Image::new();
        image.set_halign(gtk::Align::Center);
        image.set_valign(gtk::Align::Center);
        image.set_visible(false);

        // Badge de não lidas, no canto superior direito.
        let badge = gtk::Label::new(None);
        badge.add_css_class("unread-badge");
        badge.set_halign(gtk::Align::End);
        badge.set_valign(gtk::Align::Start);
        badge.set_visible(false);

        let root = gtk::Overlay::new();
        root.set_size_request(TILE, TILE);
        root.set_child(Some(&bg));
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

    /// Define a contagem de não lidas exibida no badge (0 esconde).
    pub fn set_badge(&self, count: u32) {
        if count == 0 {
            self.badge.set_visible(false);
        } else {
            let text = if count > 99 {
                "99+".to_string()
            } else {
                count.to_string()
            };
            self.badge.set_label(&text);
            self.badge.set_visible(true);
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    /// Atualiza o ícone com o favicon (ou volta à inicial se `None`).
    pub fn set_favicon(&self, texture: Option<&gdk::Texture>) {
        match texture {
            Some(tex) => {
                let native = tex.width().max(tex.height());
                // Tamanho enxuto: favicon menor, com respiro no tile.
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

/// Desenha um retângulo arredondado no contexto Cairo (path, não preenche).
fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let d = std::f64::consts::PI / 180.0;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -90.0 * d, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, 90.0 * d);
    cr.arc(x + r, y + h - r, r, 90.0 * d, 180.0 * d);
    cr.arc(x + r, y + r, r, 180.0 * d, 270.0 * d);
    cr.close_path();
}
