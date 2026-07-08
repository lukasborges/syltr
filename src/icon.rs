//! Ícone de um serviço no rail lateral.
//!
//! Renderiza um "tile" arredondado com padding e cor de fundo extraída do
//! próprio favicon (média ponderada pela opacidade dos pixels). O favicon é
//! desenhado no tamanho nativo — nunca ampliado de forma agressiva —, então
//! não fica borrado como quando um favicon 16px é esticado para 40px.
//!
//! Enquanto o favicon não carregou, mostra a inicial do nome sobre uma cor
//! derivada do nome (estável entre execuções).

use std::cell::Cell;
use std::rc::Rc;

use gtk::cairo;
use gtk::gdk;
use gtk::prelude::*;

const TILE: i32 = 40;
const RADIUS: f64 = 11.0;
/// Opacidade do fundo: um leve tint da cor do ícone, para o ícone (em cor
/// cheia) sempre se destacar — ícones de cor única não "somem" no fundo.
const BG_ALPHA: f64 = 0.18;

#[derive(Clone)]
pub struct ServiceIcon {
    root: gtk::Overlay,
    bg: gtk::DrawingArea,
    image: gtk::Image,
    label: gtk::Label,
    badge: gtk::Label,
    base_color: (f64, f64, f64),
    color: Rc<Cell<(f64, f64, f64)>>,
}

impl ServiceIcon {
    pub fn new(name: &str) -> Self {
        let base_color = hashed_color(name);
        let color = Rc::new(Cell::new(base_color));

        let bg = gtk::DrawingArea::new();
        bg.set_content_width(TILE);
        bg.set_content_height(TILE);
        {
            let color = color.clone();
            bg.set_draw_func(move |_, cr, w, h| {
                let (r, g, b) = color.get();
                rounded_rect(cr, 0.5, 0.5, w as f64 - 1.0, h as f64 - 1.0, RADIUS);
                cr.set_source_rgba(r, g, b, BG_ALPHA);
                let _ = cr.fill();
            });
        }

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
            bg,
            image,
            label,
            badge,
            base_color,
            color,
        }
    }

    /// Define a contagem de não lidas exibida no badge (0 esconde).
    pub fn set_badge(&self, count: u32) {
        if count == 0 {
            self.badge.set_visible(false);
        } else {
            let text = if count > 99 { "99+".to_string() } else { count.to_string() };
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
                // Tamanho de exibição enxuto (favicon menor, mais respiro no tile).
                let size = native.clamp(16, 22);
                self.image.set_pixel_size(size);
                self.image.set_paintable(Some(tex));
                if let Some(c) = tile_color(tex) {
                    self.color.set(c);
                }
                self.image.set_visible(true);
                self.label.set_visible(false);
            }
            None => {
                self.image.set_paintable(None::<&gdk::Texture>);
                self.color.set(self.base_color);
                self.image.set_visible(false);
                self.label.set_visible(true);
            }
        }
        self.bg.queue_draw();
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

/// Cor de fundo do tile a partir do favicon: média dos pixels ponderada pela
/// opacidade, normalizada para uma saturação/luminosidade agradáveis (evita
/// tiles quase brancos ou escuros demais).
fn tile_color(tex: &gdk::Texture) -> Option<(f64, f64, f64)> {
    let w = tex.width().max(0) as usize;
    let h = tex.height().max(0) as usize;
    if w == 0 || h == 0 {
        return None;
    }
    let stride = w * 4;
    let mut buf = vec![0u8; stride * h];
    tex.download(&mut buf, stride);

    let (mut rs, mut gs, mut bs, mut a_sum) = (0f64, 0f64, 0f64, 0f64);
    for px in buf.chunks_exact(4) {
        // Cairo ARGB32 pré-multiplicado, little-endian: B, G, R, A.
        rs += px[2] as f64;
        gs += px[1] as f64;
        bs += px[0] as f64;
        a_sum += px[3] as f64;
    }
    if a_sum <= 0.0 {
        return None;
    }
    // Como é pré-multiplicado, soma/soma_alfa já dá a média real em 0..1.
    let (r, g, b) = (
        (rs / a_sum).clamp(0.0, 1.0),
        (gs / a_sum).clamp(0.0, 1.0),
        (bs / a_sum).clamp(0.0, 1.0),
    );

    let (hue, sat, lum) = rgb_to_hsl(r, g, b);
    Some(hsl_to_rgb(hue, sat.max(0.30), lum.clamp(0.38, 0.62)))
}

fn hashed_color(text: &str) -> (f64, f64, f64) {
    let mut h: u32 = 5381;
    for byte in text.bytes() {
        h = h.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hsl_to_rgb((h % 360) as f64, 0.50, 0.55)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (r1 + m, g1 + m, b1 + m)
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h * 360.0, s, l)
}
