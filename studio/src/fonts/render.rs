//! The sample image: Pango markup for the palette and fonts, and the
//! background jobs that draw it with ImageMagick.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use crate::core::color::Rgb;
use crate::core::palette::Palette;

use super::presets::Font;

/// The X server's DPI (Xft.dpi), so the sample is the size the fonts really are.
pub fn screen_dpi() -> u32 {
    Command::new("xrdb")
        .arg("-query")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|l| l.strip_prefix("Xft.dpi:").map(|v| v.trim().to_string()))
        })
        .and_then(|v| v.parse::<f64>().ok())
        .map(|d| d.round() as u32)
        .unwrap_or(96)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Pango markup for the sample: code in the mono font, rofi rows in the ui
/// font, the bar in the bar font, all in the palette's colours, at the sizes
/// used on this machine.
/// `name` is the palette being edited (shown as the selected rofi row).
pub fn sample_markup(fonts: &[Font; 3], sizes: [u32; 3], p: &Palette, name: &str) -> String {
    let c = |k: &str| p.get(k).hex();
    let span = |k: &str, t: &str| format!("<span foreground=\"{}\">{}</span>", c(k), esc(t));
    let font = |role: usize| format!("{} {}", esc(&fonts[role].family), sizes[role]);
    let mut m = String::new();
    // mono: code, as nvim shows it
    m += &format!("<span font=\"{}\" foreground=\"{}\">", font(0), c("fg0"));
    m += &format!(
        "{} {}{}{}{}\n",
        span("purple", "local function"),
        span("blue", "apply"),
        span("grey2", "("),
        span("red", "path, opts"),
        span("grey2", ")")
    );
    m += &format!(
        "  {} {} {} {} {}\n",
        span("purple", "if"),
        span("red", "count"),
        span("fg0", "=="),
        span("orange", "0x2A"),
        span("purple", "then")
    );
    m += &format!(
        "    {}{}{}{}\n",
        span("red", "vim"),
        span("grey2", ".notify("),
        span("green", "\"theme applied 0O 1lI\""),
        span("grey2", ")")
    );
    m += &format!(
        "  {} {}\n",
        span("purple", "end"),
        span("grey1", "-- icons: \u{e0a0} \u{f07b} \u{f1d3} \u{f00af} ⇡2")
    );
    m += "</span>\n";
    // ui: the rofi theme menu
    m += "\n";
    m += &format!("<span font=\"{}\" foreground=\"{}\">", font(1), c("fg0"));
    m += &format!("Theme: {}\n", span("grey1", "type to search"));
    m += &format!(
        "<span background=\"{}\" foreground=\"{}\"> {}  </span>  Gruvbox Dark   Tokyo Night\n",
        c("yellow"),
        c("bg0"),
        esc(name)
    );
    m += "</span>\n";
    // bar: workspaces and blocks on the bar background
    m += "\n";
    m += &format!(
        "<span font=\"{}\" background=\"{}\" foreground=\"{}\"><span background=\"{}\" foreground=\"{}\"> 1 </span> 2  3        vol 73% {} 󰂯 HelixBrigitte {} 18:41 </span>",
        font(2),
        c("bg0"),
        c("fg0"),
        c("yellow"),
        c("bg0"),
        span("bg3", "│"),
        span("bg3", "│"),
    );
    m
}

/// A rendered sample, decoded (on the renderer's thread), or why it failed.
type SampleResult = std::result::Result<image::DynamicImage, String>;

/// Rendered images kept for going back to them (a few MB each).
const KEPT: usize = 8;

struct Job {
    id: u64,
    markup: String,
    bg: Rgb,
    dpi: u32,
    out: PathBuf,
    /// A fontconfig setup to use (previews of fonts that aren't installed).
    fontconfig: Option<PathBuf>,
    /// Wrap lines at this many pixels.
    width: Option<u32>,
    /// When it was asked for (for THEME_STUDIO_TIMING).
    asked: std::time::Instant,
}

/// Renders samples in the background (newest request wins), decodes them
/// there too, and keeps the last few so going back to one is instant.
pub struct SampleRenderer {
    tx: Sender<Job>,
    done: Arc<Mutex<Option<(u64, String, SampleResult)>>>,
    next: u64,
    pub last_markup: Option<String>,
    pub shown: u64,
    dir: PathBuf,
    /// File names: <prefix>-0.bmp, <prefix>-1.bmp.
    prefix: &'static str,
    /// Recent images by request, newest last.
    kept: std::collections::VecDeque<(String, image::DynamicImage)>,
    /// One from `kept`, waiting to be taken.
    ready: Option<image::DynamicImage>,
}

impl SampleRenderer {
    pub fn new(dir: &Path, dirty: Arc<AtomicBool>) -> SampleRenderer {
        Self::named(dir, dirty, "sample")
    }

    /// A renderer whose images are named <prefix>-N.bmp (several can share
    /// a folder).
    pub fn named(dir: &Path, dirty: Arc<AtomicBool>, prefix: &'static str) -> SampleRenderer {
        let (tx, rx): (Sender<Job>, Receiver<Job>) = channel();
        let done = Arc::new(Mutex::new(None));
        let d = done.clone();
        let timing = std::env::var_os("THEME_STUDIO_TIMING").is_some();
        std::thread::spawn(move || {
            while let Ok(mut job) = rx.recv() {
                // skip to the newest request
                while let Ok(newer) = rx.try_recv() {
                    job = newer;
                }
                let started = std::time::Instant::now();
                let result = render_sample(&job).and_then(|p| {
                    image::open(&p).map_err(|e| format!("reading {}: {e}", p.display()))
                });
                if timing {
                    log_timing(prefix, &job, started, &result);
                }
                let key = job_key(&job.markup, &job.fontconfig, job.width);
                *d.lock().unwrap() = Some((job.id, key, result));
                dirty.store(true, Ordering::Relaxed);
            }
        });
        SampleRenderer {
            tx,
            done,
            next: 0,
            last_markup: None,
            shown: 0,
            dir: dir.to_path_buf(),
            prefix,
            kept: Default::default(),
            ready: None,
        }
    }

    /// Asks for a new sample if the markup changed.
    pub fn request(&mut self, markup: String, bg: Rgb, dpi: u32) {
        self.request_with(markup, bg, dpi, None, None)
    }

    /// The same, with a fontconfig setup and a wrap width.
    pub fn request_with(
        &mut self,
        markup: String,
        bg: Rgb,
        dpi: u32,
        fontconfig: Option<PathBuf>,
        width: Option<u32>,
    ) {
        let key = job_key(&markup, &fontconfig, width);
        if self.last_markup.as_deref() == Some(key.as_str()) {
            return;
        }
        self.next += 1;
        // drawn before: no need to render it again
        if let Some((_, img)) = self.kept.iter().find(|(k, _)| *k == key) {
            self.ready = Some(img.clone());
            self.shown = self.next;
            self.last_markup = Some(key);
            return;
        }
        self.ready = None;
        let out = self
            .dir
            .join(format!("{}-{}.bmp", self.prefix, self.next % 2));
        let _ = self.tx.send(Job {
            id: self.next,
            markup,
            bg,
            dpi,
            out,
            fontconfig,
            width,
            asked: std::time::Instant::now(),
        });
        self.last_markup = Some(key);
    }

    /// A finished sample newer than the one shown.
    pub fn take(&mut self) -> Option<SampleResult> {
        if let Some(img) = self.ready.take() {
            return Some(Ok(img));
        }
        let mut done = self.done.lock().unwrap();
        match done.take() {
            Some((id, key, r)) if id > self.shown => {
                self.shown = id;
                drop(done);
                if let Ok(img) = &r {
                    self.kept.retain(|(k, _)| *k != key);
                    self.kept.push_back((key, img.clone()));
                    if self.kept.len() > KEPT {
                        self.kept.pop_front();
                    }
                }
                Some(r)
            }
            other => {
                *done = other;
                None
            }
        }
    }
}

/// What makes two requests the same image.
fn job_key(markup: &str, fontconfig: &Option<PathBuf>, width: Option<u32>) -> String {
    format!("{markup}{fontconfig:?}{width:?}")
}

/// THEME_STUDIO_TIMING: one line per render in
/// `~/.local/state/theme-studio/timing.log`: waiting, rendering and
/// decoding, and the size.
fn log_timing(prefix: &str, job: &Job, started: std::time::Instant, r: &SampleResult) {
    use std::io::Write as _;
    let dir = crate::core::paths::timing_log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let size = match r {
        Ok(img) => format!("{}x{}", img.width(), img.height()),
        Err(e) => format!("failed: {e}"),
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("timing.log"))
    {
        let _ = writeln!(
            f,
            "{prefix}: waited {} ms, rendered and decoded in {} ms, {size}",
            (started - job.asked).as_millis(),
            started.elapsed().as_millis()
        );
    }
}

fn render_sample(job: &Job) -> std::result::Result<PathBuf, String> {
    let markup_file = job.out.with_extension("markup");
    std::fs::write(&markup_file, &job.markup).map_err(|e| e.to_string())?;
    // at a lower priority than the studio, nvim and the shell
    let mut cmd = Command::new("nice");
    cmd.args(["-n", "5", "magick"]);
    if let Some(fc) = &job.fontconfig {
        cmd.env("FONTCONFIG_FILE", fc);
    }
    cmd.args([
        "-density",
        &job.dpi.to_string(),
        "-background",
        &job.bg.hex(),
    ]);
    if let Some(w) = job.width {
        cmd.args(["-size", &format!("{w}x")]);
    }
    let out = cmd
        .arg(format!("pango:@{}", markup_file.display()))
        .args(["-bordercolor", &job.bg.hex(), "-border", "12"])
        // 8-bit BMP: much quicker to write and read than 16-bit PNG
        .args(["-depth", "8"])
        .arg(format!("BMP3:{}", job.out.display()))
        .output()
        .map_err(|e| format!("ImageMagick: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ImageMagick: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(job.out.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::presets::Preset;

    #[test]
    fn sample_renders_with_imagemagick() {
        if Command::new("magick").arg("-version").output().is_err() {
            eprintln!("skipped: no ImageMagick");
            return;
        }
        let dir = std::env::temp_dir().join(format!("theme-studio-sample-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = Palette::blank();
        let fonts = Preset::default_preset().fonts;
        let job = Job {
            id: 1,
            markup: sample_markup(&fonts, [14, 11, 11], &p, "Test & <palette>"),
            bg: p.get("bg0"),
            dpi: 96,
            out: dir.join("s.bmp"),
            fontconfig: None,
            width: None,
            asked: std::time::Instant::now(),
        };
        let path = render_sample(&job).unwrap();
        let img = image::open(&path).unwrap();
        assert!(
            img.width() > 200 && img.height() > 100,
            "{}x{}",
            img.width(),
            img.height()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// How long a preview takes, stage by stage (run by hand:
    /// `cargo test preview_timing -- --ignored --nocapture`).
    #[test]
    #[ignore]
    #[allow(deprecated)] // a picker without a terminal to ask
    fn preview_timing() {
        use std::time::Instant;
        let p = crate::core::palette::Palette::blank();
        let st = crate::core::samples::Style {
            family: "DejaVu Sans Mono",
            size: 12,
            palette: &p,
        };
        let dir = std::env::temp_dir().join(format!("theme-studio-bench-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut picker =
            ratatui_image::picker::Picker::from_fontsize(ratatui_image::FontSize::new(10, 20));
        picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
        for (name, lines) in [("whole file", usize::MAX), ("a pane's worth", 30)] {
            let full = crate::core::samples::code(&st, 0, 0, usize::MAX);
            // keep the first `lines` lines (and the closing tag)
            let markup = if lines == usize::MAX {
                full
            } else {
                let mut m: String = full.lines().take(lines).collect::<Vec<_>>().join("\n");
                m.push_str("</span>");
                m
            };
            for run in 0..3 {
                let job = Job {
                    id: 1,
                    markup: markup.clone(),
                    bg: p.get("bg0"),
                    dpi: 96,
                    out: dir.join("bench.bmp"),
                    fontconfig: None,
                    width: None,
                    asked: Instant::now(),
                };
                let t = Instant::now();
                let out = render_sample(&job).unwrap();
                let render = t.elapsed();
                let t = Instant::now();
                let img = image::open(&out).unwrap();
                let decode = t.elapsed();
                let t = Instant::now();
                let mut proto = picker.new_resize_protocol(img.clone());
                let mut term =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 40)).unwrap();
                term.draw(|f| {
                    f.render_stateful_widget(
                        ratatui_image::StatefulImage::default()
                            .resize(ratatui_image::Resize::Crop(None)),
                        f.area(),
                        &mut proto,
                    )
                })
                .unwrap();
                let encode = t.elapsed();
                eprintln!(
                    "{name} run {run}: {}x{} render {:>4} ms, decode {:>3} ms, encode {:>3} ms, total {:>4} ms",
                    img.width(),
                    img.height(),
                    render.as_millis(),
                    decode.as_millis(),
                    encode.as_millis(),
                    (render + decode + encode).as_millis()
                );
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Going back to an image drawn before is instant (skipped without
    /// ImageMagick).
    #[test]
    fn renderer_keeps_recent_images() {
        if Command::new("magick").arg("-version").output().is_err() {
            eprintln!("skipped: no ImageMagick");
            return;
        }
        let dir = std::env::temp_dir().join(format!("theme-studio-kept-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut r = SampleRenderer::new(&dir, Arc::new(AtomicBool::new(false)));
        let bg = Rgb(0, 0, 0);
        let wait = |r: &mut SampleRenderer| {
            let t = std::time::Instant::now();
            loop {
                if let Some(x) = r.take() {
                    return x.unwrap();
                }
                assert!(t.elapsed().as_secs() < 10, "no image");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        r.request("<span>a</span>".into(), bg, 96);
        let a = wait(&mut r);
        r.request("<span>bb</span>".into(), bg, 96);
        let b = wait(&mut r);
        assert_ne!(a.width(), b.width());
        // back to the first: there at once, nothing rendered
        r.request("<span>a</span>".into(), bg, 96);
        let again = r.take().expect("kept").unwrap();
        assert_eq!(again.width(), a.width());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
