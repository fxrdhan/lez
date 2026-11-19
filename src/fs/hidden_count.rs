// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Counting of entries skipped by the visibility filters, backing the
//! `--warn-hidden` option.

/// Tracks how many entries were filtered out while listing a directory so a
/// warning can be shown afterwards.
#[derive(Debug)]
pub struct HiddenCount {
    /// Whether to show all counts regardless of how many hidden and/or
    /// ignored items there are (`--warn-hidden` given twice).
    always_print: bool,
    hidden: usize,
    ignored: usize,
}

impl HiddenCount {
    #[must_use]
    pub fn new(mode: WarnHiddenMode) -> Option<Self> {
        let always_print = match mode {
            WarnHiddenMode::Never => return None,
            WarnHiddenMode::Auto => false,
            WarnHiddenMode::Always => true,
        };

        Some(Self {
            always_print,
            hidden: 0,
            ignored: 0,
        })
    }

    pub fn inc_hidden(&mut self) {
        self.hidden += 1;
    }

    pub fn inc_ignored(&mut self) {
        self.ignored += 1;
    }

    #[must_use]
    pub(crate) fn always_print(&self) -> bool {
        self.always_print
    }

    #[must_use]
    pub(crate) fn hidden(&self) -> usize {
        self.hidden
    }

    #[must_use]
    pub(crate) fn ignored(&self) -> usize {
        self.ignored
    }
}

/// How eager `--warn-hidden` should be: never, only when something was
/// filtered out, or always printing the tally.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default)]
pub enum WarnHiddenMode {
    #[default]
    Never,
    Auto,
    Always,
}
