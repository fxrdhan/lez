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

use crate::fs::filter::SortField;
use crate::loc::{LangStat, LocCounts};
use crate::options::parser::CodeContent;
use crate::output::icons::{icon_for_name_ext, iconify_style};
use crate::theme::Theme;

/// The width, in cells, of the share bar next to the percentage column.
const BAR_WIDTH: usize = 16;

/// Eighth-block characters, from one eighth to a full block, used to give
/// the share bar sub-cell resolution.
const EIGHTHS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Format for displaying the Files column on sub-language rows.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default)]
pub enum SubFilesMode {
    #[default]
    Symbol,
    Count,
    Blank,
}

/// Options for the code-summary view.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Options {
    /// Whether to show line counts, percentages, or both.
    pub content: CodeContent,
    /// Format for displaying the Files column on sub-language rows.
    pub sub_files: SubFilesMode,
    /// Number of decimal digits to display for percentages (0..=8).
    pub percent_digits: u8,
}

/// Everything needed to render a code summary.
pub struct Render<'a> {
    pub theme: &'a Theme,
    pub opts: &'a Options,

    /// The paths to count, recursively. Empty means the current directory.
    pub roots: Vec<PathBuf>,

    /// Whether to prefix each language with a representative file icon.
    pub show_icons: bool,

    /// Whether the listing asked for hidden entries, so the walk should
    /// descend into dot-prefixed directories and count dot-prefixed files.
    pub show_hidden: bool,

    /// How languages and sub-languages should be sorted.
    pub sort_field: SortField,

    /// Whether sorting was explicitly specified by the user.
    pub is_explicit_sort: bool,

    /// Whether to reverse the sort order (ascending vs descending).
    pub reverse: bool,
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
        let report = crate::loc::count_roots(&self.roots, self.show_hidden);

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

        // Languages sorted by sort_field:
        // - By default (no explicit sort): sorted by most code first (or least code first if reverse)
        // - Explicit sort: Name (A-Z / Z-A), Size/code/percent (desc / asc), Unsorted (natural order)
        let mut langs: Vec<&LangStat> = report.languages().collect();
        if self.is_explicit_sort {
            match self.sort_field {
                SortField::Name(_)
                | SortField::NameLexicographic(_)
                | SortField::NameMixHidden(_) => {
                    if self.reverse {
                        langs.sort_by(|a, b| b.language.name.cmp(a.language.name));
                    } else {
                        langs.sort_by(|a, b| a.language.name.cmp(b.language.name));
                    }
                }
                SortField::Unsorted => {
                    if self.reverse {
                        langs.reverse();
                    }
                }
                _ => {
                    if self.reverse {
                        langs.sort_by(|a, b| {
                            a.counts
                                .code
                                .cmp(&b.counts.code)
                                .then_with(|| a.language.name.cmp(b.language.name))
                        });
                    } else {
                        langs.sort_by(|a, b| {
                            b.counts
                                .code
                                .cmp(&a.counts.code)
                                .then_with(|| a.language.name.cmp(b.language.name))
                        });
                    }
                }
            }
        } else if self.reverse {
            langs.sort_by(|a, b| {
                a.counts
                    .code
                    .cmp(&b.counts.code)
                    .then_with(|| a.language.name.cmp(b.language.name))
            });
        } else {
            langs.sort_by(|a, b| {
                b.counts
                    .code
                    .cmp(&a.counts.code)
                    .then_with(|| a.language.name.cmp(b.language.name))
            });
        }

        let total = report.total();
        let max_code = langs.iter().map(|s| s.counts.code).max().unwrap_or(0);

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
                let val = (part as f64) * 100.0 / (total.code as f64);
                let digits = self.opts.percent_digits as usize;
                format!("{val:.digits$}%")
            };
            Cell::new(text, style, Align::Right)
        };

        // Build every row up front so each column can be sized to fit.
        let mut header_row = vec![Cell::new(plain_lang("Language"), header, Align::Left)];
        let mut body: Vec<Vec<Cell>> = Vec::new();
        let mut total_row = vec![Cell::new(plain_lang("Total"), total_style, Align::Left)];

        header_row.push(Cell::new("Files".into(), header, Align::Right));

        if with_lines {
            header_row.push(Cell::new("Lines".into(), header, Align::Right));
            header_row.push(Cell::new("Code".into(), header, Align::Right));
            header_row.push(Cell::new("Comments".into(), header, Align::Right));
            header_row.push(Cell::new("Blanks".into(), header, Align::Right));
        }

        if with_percent {
            header_row.push(Cell::new("Code %".into(), header, Align::Right));
            header_row.push(Cell::new(String::new(), header, Align::Left));
        }

        let push_row = |body: &mut Vec<Vec<Cell>>,
                        label: String,
                        files_cell: Cell,
                        counts: LocCounts,
                        style: Style| {
            let mut row = vec![Cell::new(label, style, Align::Left)];
            row.push(files_cell);
            if with_lines {
                row.push(num(counts.lines, count_style));
                row.push(num(counts.code, count_style));
                row.push(num(counts.comments, dim));
                row.push(num(counts.blanks, dim));
            }
            if with_percent {
                row.push(pct(counts.code, count_style));
                row.push(Cell::new(
                    bar(counts.code, max_code),
                    bar_style,
                    Align::Left,
                ));
            }
            body.push(row);
        };

        for stat in &langs {
            push_row(
                &mut body,
                lang_cell(stat),
                num(stat.files, count_style),
                stat.counts,
                lang_style,
            );
            if !stat.embedded.is_empty() {
                let mut children: Vec<(&&str, &LangStat)> = stat.embedded.iter().collect();
                if self.is_explicit_sort {
                    match self.sort_field {
                        SortField::Name(_)
                        | SortField::NameLexicographic(_)
                        | SortField::NameMixHidden(_) => {
                            if self.reverse {
                                children.sort_by(|a, b| b.0.cmp(a.0));
                            } else {
                                children.sort_by(|a, b| a.0.cmp(b.0));
                            }
                        }
                        SortField::Unsorted => {
                            if self.reverse {
                                children.reverse();
                            }
                        }
                        _ => {
                            if self.reverse {
                                children.sort_by(|a, b| {
                                    a.1.counts
                                        .code
                                        .cmp(&b.1.counts.code)
                                        .then_with(|| a.0.cmp(b.0))
                                });
                            } else {
                                children.sort_by(|a, b| {
                                    b.1.counts
                                        .code
                                        .cmp(&a.1.counts.code)
                                        .then_with(|| a.0.cmp(b.0))
                                });
                            }
                        }
                    }
                } else if self.reverse {
                    children.sort_by(|a, b| {
                        a.1.counts
                            .code
                            .cmp(&b.1.counts.code)
                            .then_with(|| a.0.cmp(b.0))
                    });
                } else {
                    children.sort_by(|a, b| {
                        b.1.counts
                            .code
                            .cmp(&a.1.counts.code)
                            .then_with(|| a.0.cmp(b.0))
                    });
                }
                let child_count = children.len();
                for (idx, (label, child_stat)) in children.iter().enumerate() {
                    let is_last = idx == child_count - 1;
                    let tree_prefix = if self.show_icons {
                        if is_last {
                            "  └── "
                        } else {
                            "  ├── "
                        }
                    } else if is_last {
                        " └── "
                    } else {
                        " ├── "
                    };
                    let child_label = if self.show_icons {
                        let (rep_name, rep_ext) = &child_stat.rep_file;
                        let icon = icon_for_name_ext(rep_name, rep_ext.as_deref());
                        format!("{tree_prefix}{icon} {label}")
                    } else {
                        format!("{tree_prefix}{label}")
                    };
                    let files_cell = match self.opts.sub_files {
                        SubFilesMode::Count => num(child_stat.files, count_style),
                        SubFilesMode::Blank => Cell::new(String::new(), dim, Align::Right),
                        SubFilesMode::Symbol => Cell::new("*".into(), dim, Align::Right),
                    };
                    push_row(
                        &mut body,
                        child_label,
                        files_cell,
                        child_stat.counts,
                        lang_style,
                    );
                }
            }
        }

        total_row.push(num(report.total_files(), total_style));
        if with_lines {
            total_row.push(num(total.lines, total_style));
            total_row.push(num(total.code, total_style));
            total_row.push(num(total.comments, total_style));
            total_row.push(num(total.blanks, total_style));
        }
        if with_percent {
            total_row.push(pct(total.code, total_style));
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
    if !units.is_multiple_of(8) {
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
        let painted = if i == 0 {
            if let Some(rest) = cell.text.strip_prefix("  ├── ") {
                let tree = "  ├── ";
                let tree_dim = Style::default().dimmed();
                if iconify_first && rest.chars().count() > 2 {
                    let split = rest
                        .char_indices()
                        .nth(2)
                        .map_or(rest.len(), |(pos, _)| pos);
                    let (prefix, name) = rest.split_at(split);
                    format!(
                        "{}{}{}",
                        tree_dim.paint(tree),
                        iconify_style(cell.style).paint(prefix),
                        cell.style.paint(name)
                    )
                } else {
                    format!("{}{}", tree_dim.paint(tree), cell.style.paint(rest))
                }
            } else if let Some(rest) = cell.text.strip_prefix("  └── ") {
                let tree = "  └── ";
                let tree_dim = Style::default().dimmed();
                if iconify_first && rest.chars().count() > 2 {
                    let split = rest
                        .char_indices()
                        .nth(2)
                        .map_or(rest.len(), |(pos, _)| pos);
                    let (prefix, name) = rest.split_at(split);
                    format!(
                        "{}{}{}",
                        tree_dim.paint(tree),
                        iconify_style(cell.style).paint(prefix),
                        cell.style.paint(name)
                    )
                } else {
                    format!("{}{}", tree_dim.paint(tree), cell.style.paint(rest))
                }
            } else if let Some(rest) = cell.text.strip_prefix(" ├── ") {
                let tree = " ├── ";
                let tree_dim = Style::default().dimmed();
                format!("{}{}", tree_dim.paint(tree), cell.style.paint(rest))
            } else if let Some(rest) = cell.text.strip_prefix(" └── ") {
                let tree = " └── ";
                let tree_dim = Style::default().dimmed();
                format!("{}{}", tree_dim.paint(tree), cell.style.paint(rest))
            } else if iconify_first && cell.width() > 2 {
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
            }
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
