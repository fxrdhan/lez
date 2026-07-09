// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-License-Identifier: MIT
//! The **Code** view: a standalone lines-of-code summary, in the spirit of
//! tools like `tokei` and `cloc`.
//!
//! Unlike the other views it doesn’t list files at all. Instead it walks the
//! given paths (or the current directory) recursively — honouring a git
//! repository’s `.gitignore` when there is one — counts every recognised
//! source file, and prints one row per language with the project totals
//! underneath.
//!
//! It borrows eza’s long-view look: an underlined header row rather than
//! boxes, icons when they’re enabled, locale-aware number formatting, and a
//! block-character bar visualising each language’s share of the code.

use std::io::{self, Write};
use std::path::PathBuf;

use nu_ansi_term::Style;

use crate::loc::LangStat;
use crate::options::parser::CodeContent;
use crate::output::icons::{icon_for_name_ext, iconify_style};
use crate::theme::Theme;

/// The width, in cells, of the share bar next to the percentage column.
const BAR_WIDTH: usize = 16;

/// Eighth-block characters, from one eighth to a full block, used to give
/// the share bar sub-cell resolution.
const EIGHTHS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Options for the code-summary view.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Options {
    /// Whether to show line counts, percentages, or both.
    pub content: CodeContent,
}

/// Everything needed to render a code summary.
pub struct Render<'a> {
    pub theme: &'a Theme,
    pub opts: &'a Options,

    /// The paths to count, recursively. Empty means the current directory.
    pub roots: Vec<PathBuf>,

    /// Whether to prefix each language with a representative file icon.
    pub show_icons: bool,
}

/// How a summary column lines its contents up.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

/// One fully-styled cell, ready to be measured and painted.
struct Cell {
    text: String,
    style: Style,
    align: Align,
}

impl Cell {
    fn new(text: String, style: Style, align: Align) -> Self {
        Self { text, style, align }
    }

    /// The display width of this cell. Every character we emit — digits,
    /// letters, and block glyphs — occupies one terminal cell.
    fn width(&self) -> usize {
        self.text.chars().count()
    }
}

impl Render<'_> {
    pub fn render<W: Write>(self, w: &mut W) -> io::Result<()> {
        let report = crate::loc::count_roots(&self.roots);

        if report.is_empty() {
            let style = self.theme.ui.punctuation.unwrap_or_default();
            return writeln!(w, "{}", style.paint("No recognised source code found."));
        }

        let numerics =
            locale::Numeric::load_user_locale().unwrap_or_else(|_| locale::Numeric::english());

        // The eza-flavoured palette: quantities take the size colour, the
        // language names the date colour, and structure stays dim.
        let header = self.theme.ui.header.unwrap_or_default();
        let lang_style = self.theme.ui.date.unwrap_or_default();
        let count_style = self
            .theme
            .ui
            .size
            .unwrap_or_default()
            .number_byte
            .unwrap_or_default();
        let dim = self.theme.ui.punctuation.unwrap_or_default();
        let bar_style = self
            .theme
            .ui
            .filekinds
            .unwrap_or_default()
            .directory
            .unwrap_or_default();
        // Bold the totals only when colours are on at all, so piped output
        // stays free of escape codes.
        let total_style = if self.theme.ui.colourful.unwrap_or_default() {
            Style::default().bold()
        } else {
            Style::default()
        };

        let with_lines = matches!(self.opts.content, CodeContent::Lines | CodeContent::Both);
        let with_percent = matches!(self.opts.content, CodeContent::Percent | CodeContent::Both);

        // Languages sorted by most code first, then by name for stability.
        let mut langs: Vec<&LangStat> = report.languages().collect();
        langs.sort_by(|a, b| {
            b.counts
                .code
                .cmp(&a.counts.code)
                .then_with(|| a.language.name.cmp(b.language.name))
        });

        let total = report.total();
        let max_code = langs.first().map_or(0, |s| s.counts.code);

        // The icon column prefix: icons get two cells (glyph + space), and
        // every icon-less row gets two spaces so the names stay aligned.
        let lang_cell = |stat: &LangStat| {
            let name = stat.language.name;
            if self.show_icons {
                let (rep_name, rep_ext) = &stat.rep_file;
                let icon = icon_for_name_ext(rep_name, rep_ext.as_deref());
                format!("{icon} {name}")
            } else {
                name.to_string()
            }
        };
        let plain_lang = |name: &str| {
            if self.show_icons {
                format!("  {name}")
            } else {
                name.to_string()
            }
        };

        let num = |n: usize, style: Style| Cell::new(numerics.format_int(n), style, Align::Right);
        let pct = |part: usize, style: Style| {
            let text = if total.code == 0 {
                "-".to_string()
            } else {
                format!("{:.1}%", (part as f64) * 100.0 / (total.code as f64))
            };
            Cell::new(text, style, Align::Right)
        };

        // Build every row up front so each column can be sized to fit.
        let mut header_row = vec![Cell::new(plain_lang("Language"), header, Align::Left)];
        let mut body: Vec<Vec<Cell>> = langs
            .iter()
            .map(|s| vec![Cell::new(lang_cell(s), lang_style, Align::Left)])
            .collect();
        let mut total_row = vec![Cell::new(plain_lang("Total"), total_style, Align::Left)];

        header_row.push(Cell::new("Files".into(), header, Align::Right));
        for (row, s) in body.iter_mut().zip(&langs) {
            row.push(num(s.files, count_style));
        }
        total_row.push(num(report.total_files(), total_style));

        if with_lines {
            type Get = fn(&LangStat) -> usize;
            let metrics: [(&str, Get); 4] = [
                ("Lines", |s| s.counts.lines),
                ("Code", |s| s.counts.code),
                ("Comments", |s| s.counts.comments),
                ("Blanks", |s| s.counts.blanks),
            ];
            for (title, get) in metrics {
                header_row.push(Cell::new(title.into(), header, Align::Right));
                let style = if title == "Code" { count_style } else { dim };
                for (row, s) in body.iter_mut().zip(&langs) {
                    row.push(num(get(s), style));
                }
            }
            total_row.push(num(total.lines, total_style));
            total_row.push(num(total.code, total_style));
            total_row.push(num(total.comments, total_style));
            total_row.push(num(total.blanks, total_style));
        }

        if with_percent {
            header_row.push(Cell::new("Code %".into(), header, Align::Right));
            for (row, s) in body.iter_mut().zip(&langs) {
                row.push(pct(s.counts.code, count_style));
            }
            total_row.push(pct(total.code, total_style));

            // The share bar: scaled against the largest language, so the top
            // row always spans the full bar width.
            header_row.push(Cell::new(String::new(), header, Align::Left));
            for (row, s) in body.iter_mut().zip(&langs) {
                row.push(Cell::new(
                    bar(s.counts.code, max_code),
                    bar_style,
                    Align::Left,
                ));
            }
            total_row.push(Cell::new(String::new(), total_style, Align::Left));
        }

        // Size each column to its widest cell.
        let columns = header_row.len();
        let mut widths = vec![0; columns];
        for row in std::iter::once(&header_row)
            .chain(body.iter())
            .chain(std::iter::once(&total_row))
        {
            for (width, cell) in widths.iter_mut().zip(row.iter()) {
                *width = (*width).max(cell.width());
            }
        }

        writeln!(w, "{}", paint_row(&header_row, &widths, self.show_icons))?;
        for row in &body {
            writeln!(w, "{}", paint_row(row, &widths, self.show_icons))?;
        }

        let rule_width = 1 + widths.iter().sum::<usize>() + 2 * (columns - 1);
        writeln!(w, "{}", dim.paint("─".repeat(rule_width)))?;
        writeln!(w, "{}", paint_row(&total_row, &widths, self.show_icons))?;

        Ok(())
    }
}

/// Draw a bar visualising `value` against the largest value in the table,
/// with eighth-block resolution. Any non-zero value gets at least a sliver.
fn bar(value: usize, max: usize) -> String {
    if max == 0 || value == 0 {
        return String::new();
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
    let units = (((value as f64) / (max as f64)) * ((BAR_WIDTH * 8) as f64)).round() as usize;
    let units = units.max(1);

    let mut bar = "█".repeat(units / 8);
    if units % 8 > 0 {
        bar.push(EIGHTHS[units % 8 - 1]);
    }
    bar
}

/// Paint one row: a leading space, then each cell padded to its column width
/// and separated by two spaces. Trailing whitespace is trimmed so bars and
/// short final cells don’t leave invisible padding behind.
fn paint_row(cells: &[Cell], widths: &[usize], iconify_first: bool) -> String {
    let mut out = String::from(" ");
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let padding = " ".repeat(widths[i].saturating_sub(cell.width()));
        // An empty cell is pure padding: don’t emit pointless colour codes.
        if cell.text.is_empty() {
            out.push_str(&padding);
            continue;
        }
        let painted = if i == 0 && iconify_first && cell.width() > 2 {
            // Paint the icon prefix separately from the name, so underlined
            // headers don’t drag the underline through the icon column, and
            // icons keep only the colour of the style they accompany.
            let split = cell
                .text
                .char_indices()
                .nth(2)
                .map_or(cell.text.len(), |(pos, _)| pos);
            let (prefix, name) = cell.text.split_at(split);
            format!(
                "{}{}",
                iconify_style(cell.style).paint(prefix),
                cell.style.paint(name)
            )
        } else {
            cell.style.paint(cell.text.as_str()).to_string()
        };
        match cell.align {
            Align::Left => {
                out.push_str(&painted);
                out.push_str(&padding);
            }
            Align::Right => {
                out.push_str(&padding);
                out.push_str(&painted);
            }
        }
    }
    out.truncate(out.trim_end().len());
    out
}
