//! Demo ROM smoke tests: each software/*.s must assemble, run, and light pixels.

use std::fs;
use std::process::Command;

fn vintage() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vintage"))
}

fn lit_pixels(ppm: &std::path::Path) -> usize {
    let data = fs::read(ppm).unwrap();
    let header = b"P6\n256 192\n255\n";
    assert!(data.starts_with(header), "not a raw PPM");
    assert_eq!(data.len(), header.len() + 256 * 192 * 3);
    data[header.len()..]
        .chunks(3)
        .filter(|p| p != &[6, 12, 6])
        .count()
}

fn run_demo(src: &str, frames: &str) -> usize {
    let ppm = std::env::temp_dir().join(format!("vintage-demo-{}.ppm", src.trim_end_matches(".s")));
    let _ = fs::remove_file(&ppm);
    let out = vintage()
        .args(["run", &format!("software/{src}"), "--frames", frames])
        .arg("--ppm")
        .arg(&ppm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}: {}",
        src,
        String::from_utf8_lossy(&out.stderr)
    );
    lit_pixels(&ppm)
}

#[test]
fn hello_renders_banner_text() {
    // font8x8 glyphs are dense: a short banner lights hundreds of pixels
    assert!(run_demo("hello.s", "2") > 200, "text should light many pixels");
}

#[test]
fn snake_draws_playfield_and_snake() {
    // border + initial snake + food, even before any key is pressed
    assert!(run_demo("snake.s", "9") > 100, "playfield should be visible");
}

#[test]
fn cube_draws_wireframe() {
    // first draw pass lands on frame 5 (frame-tick gate), so allow 6
    assert!(run_demo("cube.s", "6") > 50, "wireframe should be visible");
}