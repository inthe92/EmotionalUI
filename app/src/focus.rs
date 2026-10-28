//! Cache exact-size focus rings, including on Slint's software renderer.
use crate::{FocusDesign, Home};
use slint::{Color, ComponentHandle, Image};
use std::{cell::RefCell, collections::HashMap};

pub fn install(ui: &Home) {
    ui.global::<FocusDesign>().on_contour(|w,h,r| {
        if !(3..=640).contains(&w) || !(3..=480).contains(&h) { return "".into(); }
        let r=r.clamp(1,w.min(h)/2);
        let a=r-1;let right=w-1;let bottom=h-1;let x=w-r;let y=h-r;
        format!("M {r} 1 H {x} A {a} {a} 0 0 1 {right} {r} V {y} A {a} {a} 0 0 1 {x} {bottom} H {r} A {a} {a} 0 0 1 1 {y} V {r} A {a} {a} 0 0 1 {r} 1 Z").into()
    });
    let cache = RefCell::new(HashMap::<_, Image>::new());
    ui.global::<FocusDesign>().on_outline(move |w, h, r, start, middle, end| {
        if !(3..=640).contains(&w) || !(3..=480).contains(&h) {
            return Image::default();
        }
        let rgb = |c: Color| [c.red(), c.green(), c.blue()];
        let colors = [rgb(start), rgb(middle), rgb(end)];
        let r = r.clamp(1, w.min(h) / 2);
        let key = (w, h, r, colors);
        let mut cache = cache.borrow_mut();
        if let Some(image) = cache.get(&key) { return image.clone(); }
        let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><defs><linearGradient id="f" x1="0" y1="0.318" x2="1" y2="0.682"><stop stop-color="{}"/><stop offset="0.5" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs><rect x="1" y="1" width="{}" height="{}" rx="{}" fill="none" stroke="url(#f)" stroke-width="2"/></svg>"##, hex(colors[0]), hex(colors[1]), hex(colors[2]), w-2, h-2, r-1);
        let image = Image::load_from_svg_data(svg.as_bytes())
            .ok()
            .and_then(|image| image.to_rgba8_premultiplied())
            .map(Image::from_rgba8_premultiplied)
            .unwrap_or_default();
        // The product uses a small fixed set of geometries; keep the bound explicit.
        if cache.len() >= 64 { cache.clear(); }
        cache.insert(key, image.clone());
        image
    });
}
