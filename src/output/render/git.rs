// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::{AnsiString as ANSIString, Style};

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};

impl f::Git {
    pub fn render(self, colours: &dyn Colours, glyphs: bool) -> TextCell {
        let staged_rendered = self.staged.render(colours, glyphs);
        let unstaged_rendered = self.unstaged.render(colours, glyphs);
        let width = DisplayWidth::from(staged_rendered.as_str())
            + DisplayWidth::from(unstaged_rendered.as_str());
        TextCell {
            width,
            contents: vec![staged_rendered, unstaged_rendered].into(),
        }
    }

    pub fn render_json(self) -> String {
        self.staged.render_json().to_owned() + self.unstaged.render_json()
    }
}

impl f::GitStatus {
    fn render(self, colours: &dyn Colours, glyphs: bool) -> ANSIString<'static> {
        let (default_letter, default_nerd_glyph, custom_glyph, style) = match self {
            Self::NotModified => (
                "-",
                "-",
                colours.not_modified_glyph(),
                colours.not_modified(),
            ),
            Self::New => ("N", "\u{f457}", colours.added_glyph(), colours.added()),
            Self::Modified => (
                "M",
                "\u{f459}",
                colours.modified_glyph(),
                colours.modified(),
            ),
            Self::Deleted => ("D", "\u{f458}", colours.deleted_glyph(), colours.deleted()),
            Self::Renamed => ("R", "\u{f45a}", colours.renamed_glyph(), colours.renamed()),
            Self::TypeChange => (
                "T",
                "\u{f471}",
                colours.type_change_glyph(),
                colours.type_change(),
            ),
            Self::Ignored => ("I", "\u{f474}", colours.ignored_glyph(), colours.ignored()),
            Self::Conflicted => (
                "U",
                "\u{f47f}",
                colours.conflicted_glyph(),
                colours.conflicted(),
            ),
        };

        if let Some(custom) = custom_glyph {
            return style.paint(custom.to_string());
        }

        if glyphs {
            style.paint(default_nerd_glyph)
        } else {
            style.paint(default_letter)
        }
    }

    fn render_json(self) -> &'static str {
        #[rustfmt::skip]
        return match self {
            Self::NotModified  => "-",
            Self::New          => "N",
            Self::Modified     => "M",
            Self::Deleted      => "D",
            Self::Renamed      => "R",
            Self::TypeChange   => "T",
            Self::Ignored      => "I",
            Self::Conflicted   => "U",
        };
    }
}

pub trait Colours {
    fn not_modified(&self) -> Style;
    fn added(&self) -> Style;
    fn modified(&self) -> Style;
    fn deleted(&self) -> Style;
    fn renamed(&self) -> Style;
    fn type_change(&self) -> Style;
    fn ignored(&self) -> Style;
    fn conflicted(&self) -> Style;

    fn not_modified_glyph(&self) -> Option<&str> {
        None
    }
    fn added_glyph(&self) -> Option<&str> {
        None
    }
    fn modified_glyph(&self) -> Option<&str> {
        None
    }
    fn deleted_glyph(&self) -> Option<&str> {
        None
    }
    fn renamed_glyph(&self) -> Option<&str> {
        None
    }
    fn type_change_glyph(&self) -> Option<&str> {
        None
    }
    fn ignored_glyph(&self) -> Option<&str> {
        None
    }
    fn conflicted_glyph(&self) -> Option<&str> {
        None
    }
}

impl f::SubdirGitRepo {
    pub fn render(self, colours: &dyn RepoColours) -> TextCell {
        let branch_name = match self.branch {
            Some(name) => {
                if self.is_worktree {
                    colours.branch_worktree().paint(name)
                } else {
                    match name.as_ref() {
                        "main" | "master" => colours.branch_main().paint(name),
                        _ => colours.branch_other().paint(name),
                    }
                }
            }
            None => colours.no_repo().paint("-"),
        };

        if let Some(status) = self.status {
            let status_rendered = status.render(colours);
            let width = DisplayWidth::from(status_rendered.as_str())
                + DisplayWidth::from(1)
                + DisplayWidth::from(branch_name.as_str());
            TextCell {
                width,
                contents: vec![status_rendered, Style::default().paint(" "), branch_name].into(),
            }
        } else {
            TextCell {
                width: DisplayWidth::from(branch_name.as_str()),
                contents: vec![branch_name].into(),
            }
        }
    }

    pub fn render_json(self) -> Option<String> {
        let branch_name = self.branch.unwrap_or("-".to_string());
        if let Some(status) = self.status {
            Some(format!("{} {}", status.render_json(), branch_name))
        } else {
            Some(branch_name)
        }
    }
}

impl f::SubdirGitRepoStatus {
    pub fn render(self, colours: &dyn RepoColours) -> ANSIString<'static> {
        match self {
            Self::NoRepo => colours
                .no_repo()
                .paint(colours.no_repo_glyph().unwrap_or("-").to_string()),
            Self::GitClean => colours
                .git_clean()
                .paint(colours.git_clean_glyph().unwrap_or("|").to_string()),
            Self::GitDirty => colours
                .git_dirty()
                .paint(colours.git_dirty_glyph().unwrap_or("+").to_string()),
        }
    }

    pub fn render_json(self) -> &'static str {
        match self {
            Self::NoRepo => "-",
            Self::GitClean => "|",
            Self::GitDirty => "+",
        }
    }
}

pub trait RepoColours {
    fn branch_main(&self) -> Style;
    fn branch_other(&self) -> Style;
    fn branch_worktree(&self) -> Style;
    fn no_repo(&self) -> Style;
    fn git_clean(&self) -> Style;
    fn git_dirty(&self) -> Style;

    fn git_clean_glyph(&self) -> Option<&str> {
        None
    }
    fn git_dirty_glyph(&self) -> Option<&str> {
        None
    }
    fn no_repo_glyph(&self) -> Option<&str> {
        None
    }
}

#[cfg(test)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;

    struct TestColours;

    impl Colours for TestColours {
        fn not_modified(&self) -> Style {
            Fixed(90).normal()
        }
        fn added(&self) -> Style {
            Fixed(91).normal()
        }
        fn modified(&self) -> Style {
            Fixed(92).normal()
        }
        fn deleted(&self) -> Style {
            Fixed(93).normal()
        }
        fn renamed(&self) -> Style {
            Fixed(94).normal()
        }
        fn type_change(&self) -> Style {
            Fixed(95).normal()
        }
        fn ignored(&self) -> Style {
            Fixed(96).normal()
        }
        fn conflicted(&self) -> Style {
            Fixed(97).normal()
        }
    }

    #[test]
    fn git_blank() {
        let stati = f::Git {
            staged: f::GitStatus::NotModified,
            unstaged: f::GitStatus::NotModified,
        };

        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(90).paint("-"), Fixed(90).paint("-")].into(),
        };

        assert_eq!(expected, stati.render(&TestColours, false));
    }

    #[test]
    fn git_new_changed() {
        let stati = f::Git {
            staged: f::GitStatus::New,
            unstaged: f::GitStatus::Modified,
        };

        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(91).paint("N"), Fixed(92).paint("M")].into(),
        };

        assert_eq!(expected, stati.render(&TestColours, false));
    }

    #[test]
    fn git_glyphs_rendering() {
        let stati = f::Git {
            staged: f::GitStatus::New,
            unstaged: f::GitStatus::Modified,
        };

        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(91).paint("\u{f457}"), Fixed(92).paint("\u{f459}")].into(),
        };

        assert_eq!(expected, stati.render(&TestColours, true));
    }

    #[test]
    fn git_blank_json() {
        let stati = f::Git {
            staged: f::GitStatus::NotModified,
            unstaged: f::GitStatus::NotModified,
        };

        let expected = "--".to_string();

        assert_eq!(expected, stati.render_json());
    }

    #[test]
    fn git_new_changed_json() {
        let stati = f::Git {
            staged: f::GitStatus::New,
            unstaged: f::GitStatus::Modified,
        };

        let expected = "NM".to_string();

        assert_eq!(expected, stati.render_json());
    }

    struct TestRepoColours;

    impl super::RepoColours for TestRepoColours {
        fn branch_main(&self) -> Style {
            Green.normal()
        }
        fn branch_other(&self) -> Style {
            Yellow.normal()
        }
        fn branch_worktree(&self) -> Style {
            Cyan.normal()
        }
        fn no_repo(&self) -> Style {
            DarkGray.normal()
        }
        fn git_clean(&self) -> Style {
            Green.normal()
        }
        fn git_dirty(&self) -> Style {
            Yellow.bold()
        }
    }

    #[test]
    fn worktree_branch_rendering() {
        let repo = f::SubdirGitRepo {
            status: Some(f::SubdirGitRepoStatus::GitClean),
            branch: Some("feat-branch".to_string()),
            is_worktree: true,
        };

        let rendered = repo.render(&TestRepoColours);
        let expected = TextCell {
            width: DisplayWidth::from(2 + "feat-branch".len()),
            contents: vec![
                Green.paint("|"),
                Style::default().paint(" "),
                Cyan.paint("feat-branch"),
            ]
            .into(),
        };
        assert_eq!(rendered, expected);
    }

    #[test]
    fn normal_branch_rendering() {
        let repo_main = f::SubdirGitRepo {
            status: Some(f::SubdirGitRepoStatus::GitClean),
            branch: Some("main".to_string()),
            is_worktree: false,
        };
        let rendered_main = repo_main.render(&TestRepoColours);
        let expected_main = TextCell {
            width: DisplayWidth::from(2 + "main".len()),
            contents: vec![
                Green.paint("|"),
                Style::default().paint(" "),
                Green.paint("main"),
            ]
            .into(),
        };
        assert_eq!(rendered_main, expected_main);

        let repo_other = f::SubdirGitRepo {
            status: Some(f::SubdirGitRepoStatus::GitDirty),
            branch: Some("feature".to_string()),
            is_worktree: false,
        };
        let rendered_other = repo_other.render(&TestRepoColours);
        let expected_other = TextCell {
            width: DisplayWidth::from(2 + "feature".len()),
            contents: vec![
                Yellow.bold().paint("+"),
                Style::default().paint(" "),
                Yellow.paint("feature"),
            ]
            .into(),
        };
        assert_eq!(rendered_other, expected_other);
    }

    struct TestCustomGlyphColours;

    impl Colours for TestCustomGlyphColours {
        fn not_modified(&self) -> Style {
            Fixed(90).normal()
        }
        fn added(&self) -> Style {
            Fixed(91).normal()
        }
        fn modified(&self) -> Style {
            Fixed(92).normal()
        }
        fn deleted(&self) -> Style {
            Fixed(93).normal()
        }
        fn renamed(&self) -> Style {
            Fixed(94).normal()
        }
        fn type_change(&self) -> Style {
            Fixed(95).normal()
        }
        fn ignored(&self) -> Style {
            Fixed(96).normal()
        }
        fn conflicted(&self) -> Style {
            Fixed(97).normal()
        }

        fn added_glyph(&self) -> Option<&str> {
            Some("✚")
        }
        fn modified_glyph(&self) -> Option<&str> {
            Some("●")
        }
    }

    #[test]
    fn git_custom_glyphs_override_standard_and_nerd_glyphs() {
        let stati = f::Git {
            staged: f::GitStatus::New,
            unstaged: f::GitStatus::Modified,
        };

        let rendered_no_flag = stati.render(&TestCustomGlyphColours, false);
        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(91).paint("✚"), Fixed(92).paint("●")].into(),
        };
        assert_eq!(rendered_no_flag, expected);

        let rendered_with_flag = stati.render(&TestCustomGlyphColours, true);
        assert_eq!(rendered_with_flag, expected);
    }

    struct TestCustomRepoGlyphColours;

    impl super::RepoColours for TestCustomRepoGlyphColours {
        fn branch_main(&self) -> Style {
            Green.normal()
        }
        fn branch_other(&self) -> Style {
            Yellow.normal()
        }
        fn branch_worktree(&self) -> Style {
            Cyan.normal()
        }
        fn no_repo(&self) -> Style {
            DarkGray.normal()
        }
        fn git_clean(&self) -> Style {
            Green.normal()
        }
        fn git_dirty(&self) -> Style {
            Yellow.bold()
        }

        fn git_clean_glyph(&self) -> Option<&str> {
            Some("✓")
        }
        fn git_dirty_glyph(&self) -> Option<&str> {
            Some("✗")
        }
    }

    #[test]
    fn git_repo_custom_glyphs_rendering() {
        let repo = f::SubdirGitRepo {
            status: Some(f::SubdirGitRepoStatus::GitClean),
            branch: Some("main".to_string()),
            is_worktree: false,
        };
        let rendered = repo.render(&TestCustomRepoGlyphColours);
        let expected = TextCell {
            width: DisplayWidth::from(1 + 1 + "main".len()),
            contents: vec![
                Green.paint("✓"),
                Style::default().paint(" "),
                Green.paint("main"),
            ]
            .into(),
        };
        assert_eq!(rendered, expected);
    }
}
