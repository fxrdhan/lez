// SPDX-FileCopyrightText: 2026 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2026 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;

use crate::{loc::Language, output::cell::TextCell};

pub trait Render {
    fn render(self, style: Style) -> TextCell;
    fn render_json(self) -> Option<String>;
}

impl Render for Option<&Language> {
    fn render(self, style: Style) -> TextCell {
        match self {
            Some(lang) => TextCell::paint(style, lang.name.to_string()),
            None => TextCell::paint(style, "-".to_string()),
        }
    }

    fn render_json(self) -> Option<String> {
        self.map(|lang| lang.name.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::loc;

    #[test]
    fn test_render_language_some() {
        let lang = loc::language_for("main.rs", Some("rs"));
        assert!(lang.is_some());
        let cell = lang.render(Style::default());
        assert_eq!(cell.strings().to_string(), "Rust");
        assert_eq!(lang.render_json(), Some("Rust".to_string()));
    }

    #[test]
    fn test_render_language_none() {
        let lang: Option<&Language> = None;
        let cell = lang.render(Style::default());
        assert_eq!(cell.strings().to_string(), "-");
        assert_eq!(lang.render_json(), None);
    }
}
