//! Render an HTML file (or a built-in page) through the Servo Compat engine
//! and write the frame as a binary PPM. No network: the page is loaded from
//! a `data:` URL and every HTTP(S) load goes to a dead loopback proxy.
//!
//! cargo run -p frihart-engine-servo --features servo --example servo_headless -- [IN.html] [OUT.ppm]

use frihart_engine::Engine;
use frihart_engine_servo::ServoEngine;
use frihart_gfx::DisplayOp;

const DEMO: &str = "<!doctype html><title>Frihart Compat</title><style>\
body{margin:0;background:#0b0b0b;color:#ffd700;font:24px sans-serif}\
main{display:grid;grid-template-columns:1fr 1fr;gap:16px;padding:24px}\
section{background:#1a1a1a;border-radius:8px;padding:16px}</style>\
<main><section>Frihart</section><section id=b>Compat engine</section></main>\
<script>document.getElementById('b').textContent='Servo ran JS: '+(6*7)</script>";

fn main() {
    let mut args = std::env::args().skip(1);
    let html = match args.next() {
        Some(p) => std::fs::read_to_string(&p).expect("read input HTML"),
        None => DEMO.to_string(),
    };
    let out = args.next().unwrap_or_else(|| "servo-headless.ppm".into());
    let t = std::time::Instant::now();
    let mut engine = ServoEngine::default();
    let r = engine.render_html(&html, 1024.0).expect("render");
    let DisplayOp::Image {
        width,
        height,
        rgba,
        ..
    } = &r.display.ops[0]
    else {
        panic!("no image");
    };
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for px in rgba.as_chunks::<4>().0 {
        ppm.extend_from_slice(&px[..3]);
    }
    std::fs::write(&out, ppm).expect("write PPM");
    println!(
        "{width}x{height} \"{}\" in {:.2}s -> {out}",
        r.title,
        t.elapsed().as_secs_f32()
    );
}
