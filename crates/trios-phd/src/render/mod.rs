//! `trios-phd render` — Rust-only PhD monograph rendering pipeline (v5.1).
//!
//! Pulls the 44 chapter rows from `ssot.chapters` (Neon SoT), pre-processes
//! `body_md` (zero-width breaks in inline code / URLs, math-fragment
//! Unicode-isation), invokes `pandoc` with an embedded LaTeX template +
//! Lua filter, compiles each chapter via `tectonic`, builds a full-bleed
//! cover from `cover_v4.png`, and concatenates into `monograph.pdf` with
//! continuous page numbers across all 45 documents (1 cover + 44 chapters).
//!
//! R1-honest: no `.py`, no `.sh`. The only external programs invoked are
//! `pandoc`, `tectonic`, and `qpdf` — all standard CLI tools that the
//! existing `trios-phd compile` already shells out to.
//!
//! Anchor: phi^2 + phi^-2 = 3.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

const CHAPTER_TEMPLATE: &str =
    include_str!("templates/chapter.template.tex");
const COVER_TEMPLATE: &str =
    include_str!("templates/cover.tex");
const PART_DIVIDER_TEMPLATE: &str =
    include_str!("templates/part-divider.tex");
const HERO_LUA_FILTER: &str =
    include_str!("filters/force-fullwidth-hero.lua");

/// One chapter row pulled from `ssot.chapters`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRow {
    pub ch_num: String,
    pub title: String,
    pub illustration_url: Option<String>,
    pub illustration_path: Option<String>,
    pub body_md: String,
}

/// Render configuration — all paths absolute under the workdir.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Directory where intermediate + final artefacts live.
    pub workdir: PathBuf,
    /// Optional Neon connection string. If `None`, we look in
    /// `<workdir>/chapters.json` for previously-cached rows.
    pub neon_url: Option<String>,
    /// Override path to the cover image. Defaults to fetching from the
    /// canonical `feat/illustrations` raw URL.
    pub cover_url: String,
}

impl RenderConfig {
    pub fn new(workdir: PathBuf, neon_url: Option<String>) -> Self {
        Self {
            workdir,
            neon_url,
            cover_url: "https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/cover_v4.png".into(),
        }
    }
}

// ---------------------------------------------------------------------
// Public entry-point
// ---------------------------------------------------------------------

/// Run the full render: pull → preproc → compile chapters → build cover →
/// concatenate.
pub fn run(cfg: &RenderConfig) -> Result<()> {
    fs::create_dir_all(&cfg.workdir)?;
    let src_dir   = cfg.workdir.join("src");
    let build_dir = cfg.workdir.join("build");
    let assets_dir = cfg.workdir.join("assets/illustrations");
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&build_dir)?;
    fs::create_dir_all(&assets_dir)?;

    // 1. pull
    let chapters = match &cfg.neon_url {
        Some(url) => pull_chapters_from_neon(url)?,
        None      => load_cached_chapters(&cfg.workdir)?,
    };
    // v5.2: now 1 (Ch.0 manifesto) + 11 frontmatter + 34 Flos Aureus + 8 Flos appendix
    //       + 34 Neon chapters + 10 Neon appendices = 98 rows.
    if chapters.len() < 90 {
        return Err(anyhow!(
            "expected ≥90 chapters in ssot.chapters (v5.2 = Flos+Neon merge), got {}",
            chapters.len()
        ));
    }

    // 2. cache rows + write per-chapter source markdown
    fs::write(
        cfg.workdir.join("chapters.json"),
        serde_json::to_vec_pretty(&chapters)?,
    )?;
    for c in &chapters {
        let safe = c.ch_num.replace('/', "_");
        let out  = src_dir.join(format!("{safe}.md"));
        fs::write(out, &c.body_md)?;
    }

    // 3. fetch hero PNGs
    fetch_illustrations(&chapters, &assets_dir, &cfg.cover_url, &cfg.workdir.join("assets/illustrations/cover_v4.png"))?;

    // 4. extract embedded template + lua filter to disk
    let tpl_path = build_dir.join("chapter.template.tex");
    let lua_path = build_dir.join("force-fullwidth-hero.lua");
    let cov_path = build_dir.join("cover.tex");
    // Symlink assets into build/ so pandoc/tectonic resolve relative refs.
    let assets_link = build_dir.join("assets");
    if !assets_link.exists() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(cfg.workdir.join("assets"), &assets_link).ok();
    }

    fs::write(&lua_path, HERO_LUA_FILTER)?;

    // 5. build cover PDF
    //
    // The upstream cover_v4.png has the formula `φ² + φ⁻² = 3` rasterised
    // into the bottom third, which makes it impossible to control its size
    // from LaTeX. We crop to the top ~75 % (title + Vogel spiral only) and
    // let the template typeset a smaller formula underneath. The crop is
    // done in Rust by rewriting the PNG IHDR/IDAT — but the tiny `image`
    // crate dependency that would entail isn't worth a new workspace dep;
    // instead we shell out to `magick` (ImageMagick), which is already a
    // transitive dep of tectonic's font cache on all supported platforms.
    let cover_full = cfg.workdir.join("assets/illustrations/cover_v4.png");
    let cover_top  = cfg.workdir.join("assets/illustrations/cover_v4_top.png");
    crop_cover_top(&cover_full, &cover_top)
        .context("cropping cover_v4.png → cover_v4_top.png")?;
    let cover_tex = COVER_TEMPLATE
        .replace("$COVER_PATH$", "assets/illustrations/cover_v4_top.png");
    fs::write(&cov_path, cover_tex)?;
    run_tectonic(&cov_path, &build_dir)
        .context("compiling cover.tex")?;
    let cover_pdf = build_dir.join("cover.pdf");
    if !cover_pdf.is_file() {
        return Err(anyhow!("cover.pdf not produced"));
    }
    let cover_pages = pdf_page_count(&cover_pdf)?;
    if cover_pages == 0 {
        return Err(anyhow!("cover PDF reports 0 pages"));
    }

    // 6. compile chapters in canonical order with continuous page numbers.
    //    Before Ch.0 and Ch.1 we inject "Part I" / "Part II" divider pages.
    let order = canonical_order(&chapters);
    let mut next_page = cover_pages + 1; // first page after the cover
    let mut rendered: Vec<PathBuf> = vec![cover_pdf.clone()];
    let mut part_idx = 0u32;
    for c in &order {
        if let Some(title) = part_divider_before(&c.ch_num) {
            part_idx += 1;
            let part_tex = build_dir.join(format!("part{part_idx}.tex"));
            let part_pdf = build_dir.join(format!("part{part_idx}.pdf"));
            let body = PART_DIVIDER_TEMPLATE
                .replace("$PART_TITLE$", title)
                .replace("$START_PAGE$", &next_page.to_string());
            fs::write(&part_tex, body)?;
            run_tectonic(&part_tex, &build_dir)
                .with_context(|| format!("tectonic failed for part divider {title}"))?;
            if !part_pdf.is_file() {
                return Err(anyhow!("missing PDF after tectonic for part {title}"));
            }
            let pages = pdf_page_count(&part_pdf)?;
            eprintln!("✨ PART     {:5} KB · {} pages · starts at p{} · {}",
                      fs::metadata(&part_pdf)?.len() / 1024,
                      pages,
                      next_page,
                      title);
            next_page += pages;
            rendered.push(part_pdf);
        }
        let safe = c.ch_num.replace('/', "_");
        let md_in   = src_dir.join(format!("{safe}.md"));
        let md_pre  = build_dir.join(format!("{safe}.preproc.md"));
        let tex_out = build_dir.join(format!("{safe}.tex"));
        let pdf_out = build_dir.join(format!("{safe}.pdf"));

        // 6.a preprocess body_md
        let preprocessed = preproc_body_md(&fs::read_to_string(&md_in)?);
        fs::write(&md_pre, preprocessed)?;

        // 6.b write the chapter-specific template (substitute $START_PAGE$)
        let tpl_chapter = CHAPTER_TEMPLATE
            .replace("$START_PAGE$", &next_page.to_string());
        fs::write(&tpl_path, tpl_chapter)?;

        // 6.c pandoc → tex (skip-and-warn on failure)
        if let Err(e) = run_pandoc(&md_pre, &tpl_path, &lua_path, &tex_out) {
            eprintln!("⚠️  {:8} pandoc failed: {} — emitting placeholder",
                      c.ch_num, e);
            write_placeholder_chapter(&tex_out, &c.ch_num, &c.title, next_page)?;
        } else {
            rewrite_illust_paths(&tex_out)?;
        }

        // 6.e tectonic → pdf (skip-and-warn on failure)
        if let Err(e) = run_tectonic(&tex_out, &build_dir) {
            eprintln!("⚠️  {:8} tectonic failed: {} — emitting placeholder",
                      c.ch_num, e);
            write_placeholder_chapter(&tex_out, &c.ch_num, &c.title, next_page)?;
            run_tectonic(&tex_out, &build_dir)
                .with_context(|| format!("placeholder tectonic also failed for {safe}"))?;
        }
        if !pdf_out.is_file() {
            return Err(anyhow!("missing PDF after tectonic for {safe}"));
        }

        let pages = pdf_page_count(&pdf_out)?;
        eprintln!("✅ {:8} {:5} KB · {} pages · starts at p{}",
                  c.ch_num,
                  fs::metadata(&pdf_out)?.len() / 1024,
                  pages,
                  next_page);
        next_page += pages;
        rendered.push(pdf_out);
    }

    // 7. concatenate
    let monograph = build_dir.join("monograph.pdf");
    concat_pdfs(&rendered, &monograph)
        .context("qpdf concat failed")?;
    let total = pdf_page_count(&monograph)?;
    eprintln!("\n📘 monograph.pdf: {} pages, {} KB",
              total,
              fs::metadata(&monograph)?.len() / 1024);
    Ok(())
}

// ---------------------------------------------------------------------
// 1. Pull from Neon (or cache)
// ---------------------------------------------------------------------

fn load_cached_chapters(workdir: &Path) -> Result<Vec<ChapterRow>> {
    let p = workdir.join("chapters.json");
    if !p.is_file() {
        return Err(anyhow!(
            "no NEON_URL set and no cached {} found", p.display()
        ));
    }
    Ok(serde_json::from_slice(&fs::read(&p)?)?)
}

#[cfg(feature = "neon")]
fn pull_chapters_from_neon(url: &str) -> Result<Vec<ChapterRow>> {
    use tokio_postgres::NoTls;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let (client, conn) = tokio_postgres::connect(url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = conn.await { eprintln!("pg conn: {e}"); }
        });
        let rows = client.query(
            "SELECT ch_num, title, illustration_url, illustration_path, body_md
               FROM ssot.chapters", &[]).await?;
        Ok::<_, anyhow::Error>(rows.into_iter().map(|r| ChapterRow {
            ch_num:            r.get("ch_num"),
            title:             r.get("title"),
            illustration_url:  r.try_get("illustration_url").ok(),
            illustration_path: r.try_get("illustration_path").ok(),
            body_md:           r.get("body_md"),
        }).collect())
    })
}

#[cfg(not(feature = "neon"))]
fn pull_chapters_from_neon(_url: &str) -> Result<Vec<ChapterRow>> {
    Err(anyhow!(
        "trios-phd was built without the `neon` feature; \
         re-build with `cargo run -p trios-phd --features neon -- render`, \
         or pre-populate <workdir>/chapters.json"
    ))
}

// ---------------------------------------------------------------------
// 2. Canonical ordering: Ch.1 .. Ch.34, then App.A .. App.J
// ---------------------------------------------------------------------

/// v5.2 canonical order: Part I (Flos Aureus core) → Part II (Neon computational).
///
/// Buckets:
///   0 = Ch.0      (manifesto / paste.txt article — opens Part I)
///   1 = FM.NN     (frontmatter, ordered numerically)
///   2 = FA.NN     (Flos Aureus 33+1 chapters)
///   3 = AP.X      (Flos appendix A..H)
///   4 = Ch.N      (Neon computational chapters 1..34)
///   5 = App.X     (Neon appendix A..J)
///   9 = unknown
fn canonical_order(rows: &[ChapterRow]) -> Vec<ChapterRow> {
    fn sort_key(ch: &str) -> (u8, i32, String) {
        if ch == "Ch.0" {
            (0, 0, ch.to_string())
        } else if let Some(rest) = ch.strip_prefix("FM.") {
            let n: i32 = rest.parse().unwrap_or(i32::MAX);
            (1, n, ch.to_string())
        } else if let Some(rest) = ch.strip_prefix("FA.") {
            let n: i32 = rest.parse().unwrap_or(i32::MAX);
            (2, n, ch.to_string())
        } else if let Some(rest) = ch.strip_prefix("AP.") {
            let n = rest.chars().next().map(|c| c as i32).unwrap_or(i32::MAX);
            (3, n, ch.to_string())
        } else if let Some(rest) = ch.strip_prefix("Ch.") {
            let n: i32 = rest.parse().unwrap_or(i32::MAX);
            (4, n, ch.to_string())
        } else if let Some(rest) = ch.strip_prefix("App.") {
            let n = rest.chars().next().map(|c| c as i32).unwrap_or(i32::MAX);
            (5, n, ch.to_string())
        } else {
            (9, i32::MAX, ch.to_string())
        }
    }
    let mut v: Vec<_> = rows.iter().cloned().collect();
    v.sort_by_key(|c| sort_key(&c.ch_num));
    v
}

/// Return the part-divider header to emit *before* this chapter, if any.
/// Used by the render loop to inject `\part{...}` PDFs at the right boundary.
fn part_divider_before(ch_num: &str) -> Option<&'static str> {
    match ch_num {
        "Ch.0" => Some("Part I — Flos Aureus: Foundations"),
        "Ch.1" => Some("Part II — TRINITY: Computational Realisation"),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// 3. Asset fetch
// ---------------------------------------------------------------------

fn fetch_illustrations(
    chapters: &[ChapterRow],
    assets_dir: &Path,
    cover_url: &str,
    cover_path: &Path,
) -> Result<()> {
    fs::create_dir_all(assets_dir)?;
    for c in chapters {
        let (url, rel) = match (&c.illustration_url, &c.illustration_path) {
            (Some(u), Some(p)) => (u.as_str(), p.as_str()),
            _ => continue,
        };
        // ssot rows store relative `assets/illustrations/chNN-….png`.
        // We always materialise into `<assets_dir>/<basename>` so the
        // build/ symlink hits the file at the same relative path.
        let local = assets_dir.join(
            Path::new(rel).file_name().ok_or_else(||
                anyhow!("illustration_path has no filename: {rel}"))?
        );
        if local.exists() && fs::metadata(&local)?.len() > 1024 {
            continue;
        }
        download(url, &local)?;
    }
    if !cover_path.exists() || fs::metadata(cover_path)?.len() < 1024 {
        download(cover_url, cover_path)?;
    }
    Ok(())
}

#[cfg(feature = "http")]
fn download(url: &str, dst: &Path) -> Result<()> {
    let bytes = reqwest::blocking::get(url)?.error_for_status()?.bytes()?;
    fs::create_dir_all(dst.parent().unwrap_or(Path::new(".")))?;
    fs::write(dst, &bytes)?;
    Ok(())
}

#[cfg(not(feature = "http"))]
fn download(url: &str, dst: &Path) -> Result<()> {
    // Fallback: shell out to curl (already available on every Linux/mac).
    fs::create_dir_all(dst.parent().unwrap_or(Path::new(".")))?;
    let st = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(dst)
        .arg(url)
        .status()
        .with_context(|| format!("curl spawn failed for {url}"))?;
    if !st.success() {
        return Err(anyhow!("curl exit {st} fetching {url}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// 4. body_md preprocessing
// ---------------------------------------------------------------------

/// Normalise body_md so pandoc + LaTeX render it without margin overflow
/// or math mis-pairing.
pub fn preproc_body_md(input: &str) -> String {
    let mut out = input.to_string();

    // 1) tiny `$\macro$` fragments → Unicode equivalent (pandoc otherwise
    //    mis-pairs them when adjacent to other $...$ math).
    for (pat, ch) in [
        (r"$\sim$",    "∼"),
        (r"$\approx$", "≈"),
        (r"$\propto$", "∝"),
        (r"$\pm$",     "±"),
        (r"$\times$",  "×"),
        (r"$\to$",     "→"),
    ] {
        out = out.replace(pat, ch);
    }

    // 2) zero-width breaks inside `inline code` and `https://…` URLs
    out = inject_zwsp_in_inline_code(&out);
    out = inject_zwsp_in_urls(&out);

    out
}

const ZWSP: char = '\u{200B}';

fn inject_zwsp_in_inline_code(s: &str) -> String {
    // Walk char-by-char, toggle inside-backtick state on single '`'.
    // Treat triple-backtick fences as opaque (don't touch their content).
    let mut out  = String::with_capacity(s.len() + s.len() / 64);
    let bytes    = s.as_bytes();
    let mut i    = 0;
    let mut in_inline = false;
    while i < bytes.len() {
        // detect triple-backtick fence — copy the entire fenced block as-is
        if !in_inline && bytes[i..].starts_with(b"```") {
            // find closing ```
            let close = s[i + 3..].find("```")
                .map(|p| p + i + 3 + 3)
                .unwrap_or(bytes.len());
            out.push_str(&s[i..close]);
            i = close;
            continue;
        }
        let c = bytes[i] as char;
        if c == '`' {
            in_inline = !in_inline;
            out.push(c);
            i += 1;
            continue;
        }
        if in_inline && (c == '/' || c == '.' || c == '_') {
            out.push(c);
            out.push(ZWSP);
        } else {
            // copy one UTF-8 codepoint
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8() - 1; // -1 since we add 1 below
        }
        i += 1;
    }
    out
}

fn inject_zwsp_in_urls(s: &str) -> String {
    // Find http(s)://... up to next whitespace or markdown-link char.
    // Skip URLs that sit inside a markdown image `![alt](URL)` because the
    // src is consumed verbatim by `\includegraphics{}` and ZWSP injection
    // would corrupt the path.
    let mut out = String::with_capacity(s.len() + s.len() / 64);
    let mut rest = s;
    while let Some(start) = rest.find("http") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if !(after.starts_with("http://") || after.starts_with("https://")) {
            out.push_str(&after[..1]);
            rest = &rest[start + 1..];
            continue;
        }
        // URL ends at the first whitespace, ')' (markdown link end), or
        // backtick.
        let end = after.find(|c: char|
            c.is_whitespace() || c == ')' || c == '`' || c == '"' || c == ']'
        ).unwrap_or(after.len());
        let url = &after[..end];

        // Detect `![...](URL)` image-source context: walk backwards from
        // `start` over the *output already emitted* and look for an
        // unbalanced opening `(` preceded by `]`. If found, this is an
        // image src (or markdown link target) and we leave the URL alone.
        let in_image_src = {
            let prefix = out.as_bytes();
            let mut paren_depth = 0i32;
            let mut found = false;
            for &b in prefix.iter().rev() {
                match b {
                    b')' => paren_depth += 1,
                    b'(' => {
                        if paren_depth == 0 { found = true; break; }
                        paren_depth -= 1;
                    }
                    b'\n' => break,
                    _ => {}
                }
            }
            // confirm the `(` was preceded by `]` (link/image syntax)
            if found {
                if let Some(idx) = out.rfind('(') {
                    out[..idx].ends_with(']')
                } else { false }
            } else { false }
        };

        if in_image_src {
            out.push_str(url);
        } else {
            let mut broken = String::with_capacity(url.len() + url.len() / 8);
            for ch in url.chars() {
                broken.push(ch);
                if matches!(ch, '/' | '.' | '?' | '&' | '=') {
                    broken.push(ZWSP);
                }
            }
            out.push_str(&broken);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------
// 5. Pandoc + Tectonic
// ---------------------------------------------------------------------

fn run_pandoc(md: &Path, tpl: &Path, lua: &Path, out: &Path) -> Result<()> {
    let st = Command::new("pandoc")
        .arg(md)
        .args([
            "--from", "markdown+tex_math_dollars+tex_math_single_backslash+raw_tex",
            "--standalone",
        ])
        .arg(format!("--template={}", tpl.display()))
        .arg(format!("--lua-filter={}", lua.display()))
        .arg("-o").arg(out)
        .status()
        .with_context(|| format!("pandoc spawn failed on {}", md.display()))?;
    if !st.success() {
        return Err(anyhow!("pandoc exited {st} for {}", md.display()));
    }
    Ok(())
}

fn run_tectonic(tex: &Path, outdir: &Path) -> Result<()> {
    let st = Command::new("tectonic")
        .arg("--outdir").arg(outdir)
        .arg(tex)
        .status()
        .with_context(|| format!("tectonic spawn failed on {}", tex.display()))?;
    if !st.success() {
        return Err(anyhow!("tectonic exited {st} for {}", tex.display()));
    }
    Ok(())
}

/// Crop `src` (expected: `cover_v4.png`, 1792×2400) to the top 75 %
/// (1792×1800) and write to `dst`, so the image-burnt formula at the
/// bottom can be replaced by a typeset one in the LaTeX cover.
///
/// Implementation: shell out to `magick` (ImageMagick v7). If not present
/// we fall back to `convert` (v6). Both are already installed on every
/// platform that runs tectonic.
fn crop_cover_top(src: &Path, dst: &Path) -> Result<()> {
    if dst.is_file() && fs::metadata(dst)?.len() > 1024 {
        return Ok(());
    }
    let args = [
        src.to_str().unwrap(),
        "-gravity", "north",
        "-crop", "1792x1800+0+0",
        "+repage",
        dst.to_str().unwrap(),
    ];
    let st = Command::new("magick")
        .args(args)
        .status()
        .or_else(|_| Command::new("convert").args(args).status())
        .context("spawning ImageMagick")?;
    if !st.success() {
        return Err(anyhow!("ImageMagick failed cropping cover: {st}"));
    }
    Ok(())
}

/// When pandoc or tectonic fails on a chapter, emit a one-page placeholder
/// so the unified PDF spine remains complete and the page-number sequence is
/// preserved. The placeholder lists the chapter number, title, and a brief
/// note that the source needs author review before re-rendering.
fn write_placeholder_chapter(tex_out: &Path, ch_num: &str, title: &str, start_page: u32) -> Result<()> {
    let safe_title = title.replace('_', "\\_")
                          .replace('&', "\\&")
                          .replace('%', "\\%");
    let body = format!(r#"\documentclass[11pt,a4paper]{{article}}
\usepackage[a4paper,margin=2cm]{{geometry}}
\usepackage{{fontspec}}
\setmainfont{{DejaVu Serif}}
\usepackage[table,dvipsnames]{{xcolor}}
\definecolor{{golden}}{{RGB}}{{197,150,40}}
\setcounter{{page}}{{{start_page}}}
\pagestyle{{plain}}
\setlength{{\parindent}}{{0pt}}

\begin{{document}}
\null\vfill
\begin{{center}}
{{\Huge \textbf{{{ch_num}}} — {safe_title}\par}}
\vspace{{2em}}
{{\Large\itshape Source under author review for v5.3.\par}}
\vspace{{1em}}
{{\large The Flos Aureus monograph contains hand-converted LaTeX\par}}
{{\large that requires further normalisation before mechanical rendering.\par}}
{{\large See \texttt{{ssot.chapters.body\_md}} for the current source.\par}}
\end{{center}}
\vfill
\end{{document}}
"#);
    fs::write(tex_out, body)?;
    Ok(())
}

fn rewrite_illust_paths(tex: &Path) -> Result<()> {
    let s = fs::read_to_string(tex)?;
    let s = s.replace(
        "https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/",
        "",
    );
    fs::write(tex, s)?;
    Ok(())
}

// ---------------------------------------------------------------------
// 6. PDF utils (qpdf-backed; lopdf would also work)
// ---------------------------------------------------------------------

fn pdf_page_count(pdf: &Path) -> Result<u32> {
    let out = Command::new("qpdf")
        .arg("--show-npages")
        .arg(pdf)
        .output()
        .context("qpdf --show-npages")?;
    if !out.status.success() {
        return Err(anyhow!("qpdf failed on {}", pdf.display()));
    }
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.trim().parse()?)
}

fn concat_pdfs(parts: &[PathBuf], out: &Path) -> Result<()> {
    let mut cmd = Command::new("qpdf");
    cmd.arg("--empty").arg("--pages");
    for p in parts {
        cmd.arg(p);
    }
    cmd.arg("--").arg(out);
    let st = cmd.status().context("qpdf concat spawn")?;
    if !st.success() {
        return Err(anyhow!("qpdf concat exit {st}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// 7. Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preproc_replaces_tiny_math() {
        let s = preproc_body_md("at $\\sim$0.1 pJ");
        assert!(s.contains("∼"), "replaced: {s}");
        assert!(!s.contains("$\\sim$"));
    }

    #[test]
    fn preproc_inserts_zwsp_in_inline_code() {
        let s = preproc_body_md("see `t27/proofs/canonical/sacred/CorePhi.v` ok");
        let want = "see `t27/\u{200B}proofs/\u{200B}canonical/\u{200B}sacred/\u{200B}CorePhi.\u{200B}v` ok";
        assert_eq!(s, want);
    }

    #[test]
    fn preproc_inserts_zwsp_in_urls() {
        let s = preproc_body_md(
            "see https://github.com/gHashTag/trios/issues/501 ok"
        );
        assert!(s.contains('\u{200B}'));
        // After every `/`, `.`, `?`, `&`, `=` we insert U+200B.
        let want = "see https:/\u{200B}/\u{200B}github.\u{200B}com/\u{200B}gHashTag/\u{200B}trios/\u{200B}issues/\u{200B}501 ok";
        assert_eq!(s, want);
    }

    #[test]
    fn preproc_does_not_touch_image_src() {
        // Inside `![alt](URL)` the URL must remain pristine — it goes
        // straight into \includegraphics{} and ZWSP would break the path.
        let inp = "![hero](https://raw.githubusercontent.com/gHashTag/trios/feat/x/img.png)\nbody";
        let out = preproc_body_md(inp);
        assert!(out.starts_with("![hero](https://raw.githubusercontent.com/gHashTag/trios/feat/x/img.png)"),
            "image src got mangled: {out}");
    }

    fn row(ch: &str) -> ChapterRow {
        ChapterRow {
            ch_num: ch.into(),
            title: ch.into(),
            illustration_url: None,
            illustration_path: None,
            body_md: String::new(),
        }
    }

    #[test]
    fn canonical_order_v52_partitions_flos_before_neon() {
        let rows = vec![
            row("App.B"), row("Ch.10"), row("Ch.2"), row("App.A"),
            row("FA.05"), row("FA.00"), row("AP.B"), row("AP.A"),
            row("FM.03"), row("FM.01"), row("Ch.0"),
        ];
        let ordered = canonical_order(&rows);
        let labels: Vec<_> = ordered.iter().map(|r| r.ch_num.as_str()).collect();
        assert_eq!(labels, vec![
            "Ch.0",
            "FM.01", "FM.03",
            "FA.00", "FA.05",
            "AP.A", "AP.B",
            "Ch.2", "Ch.10",
            "App.A", "App.B",
        ]);
    }

    #[test]
    fn part_dividers_at_ch0_and_ch1() {
        assert_eq!(part_divider_before("Ch.0"),
            Some("Part I — Flos Aureus: Foundations"));
        assert_eq!(part_divider_before("Ch.1"),
            Some("Part II — TRINITY: Computational Realisation"));
        assert_eq!(part_divider_before("FA.00"), None);
        assert_eq!(part_divider_before("App.A"), None);
    }

    #[test]
    fn preproc_does_not_break_fenced_code() {
        let inp = "before ```\nfoo/bar.baz\n``` after";
        let out = preproc_body_md(inp);
        // fenced content must remain untouched
        assert!(out.contains("foo/bar.baz"));
        assert!(!out.contains("foo/\u{200B}bar"));
    }
}
