// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Presentation of filtered-entry tallies for `--warn-hidden`.

use nu_ansi_term::Style;

pub use crate::fs::hidden_count::{HiddenCount, WarnHiddenMode};

impl HiddenCount {
    /// The warning line to print, if any: in `Auto` mode nothing is produced
    /// until something was actually filtered out.
    #[must_use]
    pub fn render(&self, style: Style) -> Option<String> {
        // A lone count says "1 hidden item"; where two counts share the
        // noun, it follows the nearer one, which a tally of one item in all
        // leaves at zero.
        let items = |count: usize| if count == 1 { "item" } else { "items" };
        let warn_string = match (self.always_print(), self.hidden(), self.ignored()) {
            (false, 0, 0) => None,
            (false, hidden, 0) => Some(format!("...and {hidden} hidden {}", items(hidden))),
            (false, 0, ignored) => Some(format!("...and {ignored} ignored {}", items(ignored))),
            (false, hidden, ignored) => Some(format!(
                "...and {hidden} hidden, {ignored} ignored {}",
                items(ignored)
            )),
            (true, hidden, ignored) => Some(format!(
                "{hidden} hidden and {ignored} ignored {}",
                items(ignored)
            )),
        };
        warn_string.map(|s| style.paint(s).to_string())
    }
}
