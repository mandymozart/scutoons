//! Text on the CPU (S3): a chapter's dialog set into its slice the way the reader places it (shared/reader dialog.svelte:
//! rect in % of the slice, `size` = % of the slice width, shrunk to fit a box with a height, paper boxes for captions /
//! narration / framed bubbles, a white field behind a plain bubble, coloured + stroked lettering, the rotation) and the
//! big caption the overlay node shows. The reader's fonts: Bangers ('lettering') and IBM Plex Mono ('mono'), SIL OFL
//! (assets/*-OFL.txt). Not drawn: bubble tails, wobbly shapes, gradients – the reader's SVG work.
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use serde::Deserialize;
use std::sync::OnceLock;

static BANGERS: &[u8] = include_bytes!("../assets/Bangers-latin.ttf");
static MONO: &[u8] = include_bytes!("../assets/IBMPlexMono-Bold-latin.ttf");

fn font(name: &str) -> &'static FontRef<'static> {
    static F: OnceLock<(FontRef<'static>, FontRef<'static>)> = OnceLock::new();
    let (lettering, mono) = F.get_or_init(|| {
        (FontRef::try_from_slice(BANGERS).expect("Bangers"), FontRef::try_from_slice(MONO).expect("IBM Plex Mono"))
    });
    if name == "lettering" { lettering } else { mono }
}

/// The deck's mono face (IBM Plex Mono Bold): what the deck's own UI is painted in (deck-kit `raster`, #481).
pub fn mono() -> &'static FontRef<'static> {
    font("mono")
}

/// One `Slice.dialog` entry (shared/content `Dialog`) – the fields the stage draws.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Dialog {
    pub rect: Rect,
    /// a string, or a map locale → string (the first wins)
    pub text: serde_json::Value,
    pub variant: Option<String>,
    pub font: Option<String>,
    pub align: Option<String>,
    pub valign: Option<String>,
    pub size: Option<f32>,
    /// a colour, or a gradient `{ type, angle, stops: [{ at, color }] }` – drawn in its first stop's colour
    #[serde(deserialize_with = "paint")]
    pub color: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub uppercase: Option<bool>,
    pub frame: Option<bool>,
    pub rotation: Option<f32>,
}

/// a colour string, or a gradient's first stop (gradients are not drawn; chapter 2's "laut" has one, #188)
fn paint<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = Option::<serde_json::Value>::deserialize(d)?;
    Ok(match v {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(g) => g.get("stops").and_then(|s| s.get(0)).and_then(|s| s.get("color")).and_then(|c| c.as_str()).map(str::to_string),
        None => None,
    })
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: Option<f32>,
}

impl Dialog {
    pub fn text(&self) -> String {
        let t = match &self.text {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(m) => m.get("de").or_else(|| m.values().next()).and_then(|v| v.as_str()).unwrap_or("").into(),
            _ => String::new(),
        };
        if self.uppercase.unwrap_or(false) { t.to_uppercase() } else { t }
    }

    fn font_name(&self) -> &str {
        self.font.as_deref().unwrap_or(if self.variant.as_deref() == Some("bubble") { "lettering" } else { "mono" })
    }

    /// captions and narration sit in a paper box unless `frame: false`; a bubble only with `frame: true`
    fn framed(&self) -> bool {
        match self.variant.as_deref() {
            Some("caption" | "narration") => self.frame != Some(false),
            _ => self.frame == Some(true),
        }
    }

    /// painted lettering (a colour, often a stroke) floats on the art without a field
    fn painted(&self) -> bool {
        self.font_name() == "lettering" && self.color.is_some()
    }
}

pub fn parse_color(s: &str) -> Option<[u8; 4]> {
    let h = s.strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some([v(0)?, v(2)?, v(4)?, 255]),
        8 => Some([v(0)?, v(2)?, v(4)?, v(6)?]),
        3 => {
            let c = |i: usize| u8::from_str_radix(&h[i..i + 1], 16).ok().map(|x| x * 17);
            Some([c(0)?, c(1)?, c(2)?, 255])
        }
        _ => None,
    }
}

/// the text broken into lines that fit `width` px (explicit newlines kept, words wrapped, an overlong word split)
pub fn wrap(font: &FontRef, px: f32, text: &str, width: f32) -> Vec<String> {
    let sf = font.as_scaled(PxScale::from(px));
    let measure = |s: &str| line_width(&sf, s);
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if measure(&cand) <= width || line.is_empty() && measure(word) <= width {
                line = cand;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            // a word wider than the box: split it by characters (the reader's overflow-wrap: anywhere)
            for ch in word.chars() {
                let mut c = line.clone();
                c.push(ch);
                if measure(&c) > width && !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(ch);
            }
        }
        lines.push(line);
    }
    lines
}

fn line_width<F: Font>(sf: &ab_glyph::PxScaleFont<&F>, s: &str) -> f32 {
    let mut w = 0.0;
    let mut prev = None;
    for c in s.chars() {
        let id = sf.glyph_id(c);
        if let Some(p) = prev {
            w += sf.kern(p, id);
        }
        w += sf.h_advance(id);
        prev = Some(id);
    }
    w
}

/// blend `rgba` with coverage `c` into the pixel
fn blend(img: &mut RgbaImage, x: i32, y: i32, rgba: [u8; 4], c: f32) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let a = (c.clamp(0.0, 1.0) * rgba[3] as f32 / 255.0).clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    let da = p[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    for k in 0..3 {
        let v = (rgba[k] as f32 * a + p[k] as f32 * da * (1.0 - a)) / oa.max(1e-6);
        p[k] = v.round() as u8;
    }
    p[3] = (oa * 255.0).round() as u8;
}

fn fill_rect(img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, rgba: [u8; 4]) {
    for y in y0.floor().max(0.0) as i32..y1.ceil().min(img.height() as f32) as i32 {
        for x in x0.floor().max(0.0) as i32..x1.ceil().min(img.width() as f32) as i32 {
            blend(img, x, y, rgba, 1.0);
        }
    }
}

/// a rounded field (the bubble's place) – an ellipse-ish rounded rectangle
fn fill_round(img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, rgba: [u8; 4]) {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    for y in y0.floor().max(0.0) as i32..y1.ceil().min(img.height() as f32) as i32 {
        for x in x0.floor().max(0.0) as i32..x1.ceil().min(img.width() as f32) as i32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = (x0 + r - px).max(px - (x1 - r)).max(0.0);
            let dy = (y0 + r - py).max(py - (y1 - r)).max(0.0);
            let d = (dx * dx + dy * dy).sqrt() - r;
            blend(img, x, y, rgba, 0.5 - d);
        }
    }
}

/// draw `lines` with their top-left at (x, y): the stroke first (the glyphs stamped around a circle), then the fill
#[allow(clippy::too_many_arguments)]
fn draw_lines(img: &mut RgbaImage, font: &FontRef, px: f32, lh: f32, lines: &[String], x: f32, y: f32, width: f32, align: &str,
              fill: [u8; 4], stroke: Option<([u8; 4], f32)>) {
    let sf = font.as_scaled(PxScale::from(px));
    let mut glyphs = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let w = line_width(&sf, line);
        let mut cx = x + match align {
            "center" => (width - w) / 2.0,
            "right" | "end" => width - w,
            _ => 0.0,
        };
        let base = y + i as f32 * lh + (lh - (sf.ascent() - sf.descent())) / 2.0 + sf.ascent();
        let mut prev = None;
        for c in line.chars() {
            let id = sf.glyph_id(c);
            if let Some(p) = prev {
                cx += sf.kern(p, id);
            }
            glyphs.push(id.with_scale_and_position(px, ab_glyph::point(cx, base)));
            cx += sf.h_advance(id);
            prev = Some(id);
        }
    }
    let mut stamp = |g: &ab_glyph::Glyph, dx: f32, dy: f32, rgba: [u8; 4]| {
        let mut g = g.clone();
        g.position.x += dx;
        g.position.y += dy;
        if let Some(o) = font.outline_glyph(g) {
            let b = o.px_bounds();
            o.draw(|gx, gy, c| blend(img, b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, rgba, c));
        }
    };
    if let Some((rgba, r)) = stroke.filter(|(_, r)| *r >= 0.5) {
        let steps = ((r * 4.0) as usize).clamp(8, 24);
        for g in &glyphs {
            for k in 0..steps {
                let a = k as f32 / steps as f32 * std::f32::consts::TAU;
                stamp(g, a.cos() * r, a.sin() * r, rgba);
            }
        }
    }
    for g in &glyphs {
        stamp(g, 0.0, 0.0, fill);
    }
}

/// Set a slice's dialog into its image (the slice's own pixel size; every measure in % of it, as in the reader).
pub fn bake_dialog(img: &mut RgbaImage, dialog: &[Dialog]) {
    let (sw, sh) = (img.width() as f32, img.height() as f32);
    for d in dialog {
        let text = d.text();
        if text.trim().is_empty() {
            continue;
        }
        let rot = d.rotation.unwrap_or(0.0);
        if rot.abs() < 0.5 {
            draw_dialog(img, d, &text, sw, sh, 0.0, 0.0);
            continue;
        }
        // turned: draw into a layer of the slice's size, then rotate it about the box centre into the slice
        let mut layer = RgbaImage::new(img.width(), img.height());
        let (cx, cy) = draw_dialog(&mut layer, d, &text, sw, sh, 0.0, 0.0);
        composite_rotated(img, &layer, cx, cy, rot.to_radians());
    }
}

/// one dialog box; returns its centre (the rotation's pivot)
fn draw_dialog(img: &mut RgbaImage, d: &Dialog, text: &str, sw: f32, sh: f32, ox: f32, oy: f32) -> (f32, f32) {
    let font = font(d.font_name());
    let lettering = d.font_name() == "lettering";
    let framed = d.framed();
    let x = ox + d.rect.x / 100.0 * sw;
    let y = oy + d.rect.y / 100.0 * sh;
    let w = (d.rect.width.max(1.0) / 100.0 * sw).min(sw - x).max(8.0);
    let base_px = d.size.unwrap_or(4.0) / 100.0 * sw;
    let lh_k = if lettering { 1.1 } else { 1.35 };
    // the paper box's padding (0.8em 1em) and border (2px at the reader's 390 px column)
    let pad = |px: f32| if framed { (px, px * 0.8) } else { (0.0, 0.0) };
    let mut px = base_px;
    let mut lines;
    loop {
        let (padx, _) = pad(px);
        lines = wrap(font, px, text, w - 2.0 * padx);
        let (_, pady) = pad(px);
        let need = lines.len() as f32 * px * lh_k + 2.0 * pady;
        match d.rect.height {
            Some(hh) if need > hh / 100.0 * sh && px > base_px * 0.4 => px *= 0.95,
            _ => break,
        }
    }
    let (padx, pady) = pad(px);
    let lh = px * lh_k;
    let text_h = lines.len() as f32 * lh;
    let box_h = d.rect.height.map(|hh| hh / 100.0 * sh).unwrap_or(text_h + 2.0 * pady);
    let ty = y + match d.valign.as_deref() {
        Some("middle" | "center") | None if d.rect.height.is_some() => (box_h - text_h) / 2.0,
        Some("bottom" | "end") => box_h - text_h - pady,
        _ => pady,
    };
    let border = (2.0 * sw / 390.0).max(1.0);
    let ink = [0, 0, 0, 255];
    let paper = [255, 255, 255, 255];
    if framed {
        fill_rect(img, x, y, x + w, y + box_h, ink);
        fill_rect(img, x + border, y + border, x + w - border, y + box_h - border, paper);
    } else if !d.painted() && d.variant.as_deref() == Some("bubble") {
        // the reader draws a bubble shape here; the stage a rounded paper field around the text
        let (gx, gy) = (px * 0.5, px * 0.35);
        fill_round(img, x - gx, ty - gy, x + w + gx, ty + text_h + gy, px * 1.2, paper);
    }
    let fill = d.color.as_deref().and_then(parse_color).unwrap_or(ink);
    let stroke = d.stroke.as_deref().and_then(parse_color).map(|c| (c, d.stroke_width.unwrap_or(0.0) * px * 0.6));
    let align = d.align.as_deref().unwrap_or("left");
    draw_lines(img, font, px, lh, &lines, x + padx, ty, w - 2.0 * padx, align, fill, stroke);
    (x + w / 2.0, y + box_h / 2.0)
}

/// `layer` turned by `angle` (radians, clockwise like CSS) about (cx, cy), blended over `img` (bilinear)
fn composite_rotated(img: &mut RgbaImage, layer: &RgbaImage, cx: f32, cy: f32, angle: f32) {
    let (s, c) = angle.sin_cos();
    let (w, h) = (layer.width() as i32, layer.height() as i32);
    let sample = |x: f32, y: f32| -> [f32; 4] {
        let (x0, y0) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let px = |x: i32, y: i32| -> [f32; 4] {
            if x < 0 || y < 0 || x >= w || y >= h {
                return [0.0; 4];
            }
            let p = layer.get_pixel(x as u32, y as u32);
            let a = p[3] as f32;
            [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, a] // premultiplied for the filter
        };
        let (a, b, cc, d) = (px(x0, y0), px(x0 + 1, y0), px(x0, y0 + 1), px(x0 + 1, y0 + 1));
        let mut o = [0.0; 4];
        for k in 0..4 {
            o[k] = (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (cc[k] * (1.0 - fx) + d[k] * fx) * fy;
        }
        o
    };
    for y in 0..img.height() {
        for x in 0..img.width() {
            // inverse rotation: where in the layer this pixel comes from
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let (sx, sy) = (c * dx + s * dy + cx - 0.5, -s * dx + c * dy + cy - 0.5);
            let o = sample(sx, sy);
            if o[3] <= 0.5 {
                continue;
            }
            let a = o[3] / 255.0;
            let rgba = [(o[0] / o[3]) as u8, (o[1] / o[3]) as u8, (o[2] / o[3]) as u8, 255];
            blend(img, x as i32, y as i32, rgba, a);
        }
    }
}

/// The big caption the overlay shows: up to `max_lines` lines in Bangers, white with a black stroke, on transparent,
/// `width` px wide (the text centred, the height as the lines need).
pub fn caption(text: &str, width: u32, max_lines: usize) -> Option<RgbaImage> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let f = font("lettering");
    let margin = width as f32 * 0.04;
    let mut px = width as f32 / 7.0;
    let mut lines = wrap(f, px, text, width as f32 - 2.0 * margin);
    while lines.len() > max_lines && px > width as f32 / 24.0 {
        px *= 0.9;
        lines = wrap(f, px, text, width as f32 - 2.0 * margin);
    }
    lines.truncate(max_lines);
    let lh = px * 1.05;
    let h = (lines.len() as f32 * lh + 2.0 * margin).ceil() as u32;
    let mut img = RgbaImage::from_pixel(width, h, Rgba([255, 255, 255, 0]));
    draw_lines(&mut img, f, px, lh, &lines, margin, margin, width as f32 - 2.0 * margin, "center", [255, 255, 255, 255],
               Some(([0, 0, 0, 255], px * 0.06)));
    Some(img)
}

/// A Live outlet's text layer (#186): `text` in Bangers at `px` (the line height), wrapped to `width`, centred, in
/// `fill` with a black stroke, on transparent – at most `max_h` px high (the lines that don't fit are dropped).
pub fn text_lines(text: &str, width: u32, px: f32, fill: [u8; 4], max_h: u32) -> Option<RgbaImage> {
    let text = text.trim();
    if text.is_empty() || width < 8 || px < 4.0 {
        return None;
    }
    let f = font("lettering");
    let margin = px * 0.25;
    let lh = px * 1.05;
    let mut lines = wrap(f, px, text, width as f32 - 2.0 * margin);
    let fit = (((max_h as f32 - 2.0 * margin) / lh).floor() as usize).max(1);
    lines.truncate(fit);
    let h = ((lines.len() as f32 * lh + 2.0 * margin).ceil() as u32).min(max_h.max(1));
    let mut img = RgbaImage::from_pixel(width, h, Rgba([fill[0], fill[1], fill[2], 0]));
    draw_lines(&mut img, f, px, lh, &lines, margin, margin, width as f32 - 2.0 * margin, "center", fill, Some(([0, 0, 0, fill[3]], px * 0.06)));
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlet_text_wraps_into_its_box() {
        let img = text_lines("Liebe Gemeinde, die Predigt steht bevor", 300, 40.0, [255, 200, 0, 255], 400).unwrap();
        assert_eq!(img.width(), 300);
        assert!(img.height() > 80, "two lines at least: {}", img.height());
        assert!(img.pixels().any(|p| p.0 == [255, 200, 0, 255]), "the fill colour");
        let short = text_lines("Liebe Gemeinde, die Predigt steht bevor", 300, 40.0, [255; 4], 60).unwrap();
        assert!(short.height() <= 60, "clipped to its box");
        assert!(text_lines("  ", 300, 40.0, [255; 4], 60).is_none());
    }

    #[test]
    fn colours_parse() {
        assert_eq!(parse_color("#ffffff"), Some([255, 255, 255, 255]));
        assert_eq!(parse_color("#000"), Some([0, 0, 0, 255]));
        assert_eq!(parse_color("#11223380"), Some([0x11, 0x22, 0x33, 0x80]));
        assert_eq!(parse_color("red"), None);
    }

    #[test]
    fn wrapping_keeps_newlines_and_fits_the_width() {
        let f = font("mono");
        let lines = wrap(f, 20.0, "EIN NEUER TAG,\nEIN ALTES SPIEL!", 1000.0);
        assert_eq!(lines, vec!["EIN NEUER TAG,", "EIN ALTES SPIEL!"]);
        let narrow = wrap(f, 20.0, "An jedem Morgen ging die Sonne auf", 120.0);
        assert!(narrow.len() > 2, "{narrow:?}");
        let sf = f.as_scaled(PxScale::from(20.0));
        assert!(narrow.iter().all(|l| line_width(&sf, l) <= 120.0 + 0.5), "{narrow:?}");
    }

    #[test]
    fn dialog_is_set_into_the_slice() {
        let json = r##"[{"rect":{"x":10,"y":10,"width":80,"height":20},"text":"EIN NEUER TAG","variant":"narration","size":6,
            "font":"mono","valign":"middle"},
            {"rect":{"x":10,"y":60,"width":70},"text":"Leugnen","variant":"bubble","font":"lettering","size":14,
            "color":"#ffffff","stroke":"#000000","strokeWidth":0.1,"rotation":-7}]"##;
        let gradient: Vec<Dialog> = serde_json::from_str(r##"[{"text":"laut","color":{"type":"linear","angle":180,"stops":[{"at":0,"color":"#ffc531"},{"at":100,"color":"#00ff94"}]}}]"##).unwrap();
        assert_eq!(gradient[0].color.as_deref(), Some("#ffc531"), "a gradient: its first stop");
        let d: Vec<Dialog> = serde_json::from_str(json).unwrap();
        let mut img = RgbaImage::from_pixel(390, 624, Rgba([40, 80, 120, 255]));
        bake_dialog(&mut img, &d);
        // the caption's paper box (white) with its ink border, and white lettering further down
        assert_eq!(img.get_pixel(45, 70).0, [255, 255, 255, 255], "paper inside the box");
        assert_eq!(img.get_pixel(39, 62).0, [0, 0, 0, 255], "the border");
        let lettering = (360..470).flat_map(|y| (40..330).map(move |x| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 == [255, 255, 255, 255]).count();
        assert!(lettering > 200, "white lettering pixels: {lettering}");
    }

    #[test]
    fn a_caption_is_white_on_transparent() {
        let img = caption("ZZZZZ", 800, 2).unwrap();
        assert_eq!(img.width(), 800);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert!(img.pixels().any(|p| p.0 == [255, 255, 255, 255]));
        assert!(caption("  ", 800, 2).is_none());
    }
}
