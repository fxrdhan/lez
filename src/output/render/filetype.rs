// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::{AnsiString as ANSIString, Style};

use crate::fs::fields as f;
use crate::theme::LinkStyle;

impl f::Type {
    pub fn render<C: Colours>(self, colours: &C, mount: bool) -> ANSIString<'static> {
        #[rustfmt::skip]
        return match self {
            Self::File         => colours.normal().paint("."),
            Self::Directory    => colours.directory().paint(if mount { "D" } else { "d" }),
            Self::Pipe         => colours.pipe().paint("|"),
            Self::Link         => match colours.symlink() {
                LinkStyle::AnsiStyle(style) => style.paint("l"),
                // With ln=target the indicator has no colour of its own; it
                // borrows nothing here since the type char is shared.
                LinkStyle::Target           => colours.normal().paint("l"),
            },
            Self::BlockDevice  => colours.block_device().paint("b"),
            Self::CharDevice   => colours.char_device().paint("c"),
            Self::Socket       => colours.socket().paint("s"),
            Self::Special      => colours.special().paint("?"),
        };
    }

    pub fn render_json(self) -> &'static str {
        #[rustfmt::skip]
        return match self {
            Self::File         => ".",
            Self::Directory    => "d",
            Self::Pipe         => "|",
            Self::Link         => "l",
            Self::BlockDevice  => "b",
            Self::CharDevice   => "c",
            Self::Socket       => "s",
            Self::Special      => "?",
        };
    }
}

use crate::fs::fields::TagColor;

pub trait Colours {
    fn normal(&self) -> Style;
    fn directory(&self) -> Style;
    fn pipe(&self) -> Style;
    fn symlink(&self) -> LinkStyle;
    fn block_device(&self) -> Style;
    fn char_device(&self) -> Style;
    fn socket(&self) -> Style;
    fn special(&self) -> Style;
    fn tag(&self, tag: &TagColor) -> Style;
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_ansi_term::Color::*;

    struct DummyColours;

    #[rustfmt::skip]
    impl Colours for DummyColours {
        fn normal(&self)       -> Style { Fixed(1).normal() }
        fn directory(&self)    -> Style { Fixed(2).bold() }
        fn pipe(&self)         -> Style { Fixed(3).normal() }
        fn symlink(&self)      -> LinkStyle { LinkStyle::AnsiStyle(Fixed(4).normal()) }
        fn block_device(&self) -> Style { Fixed(5).normal() }
        fn char_device(&self)  -> Style { Fixed(6).normal() }
        fn socket(&self)       -> Style { Fixed(7).normal() }
        fn special(&self)      -> Style { Fixed(8).normal() }
        fn tag(&self, _tag: &TagColor) -> Style { Fixed(9).normal() }
    }

    #[test]
    fn test_filetype_render_regular_directory() {
        let colours = DummyColours;
        let rendered = f::Type::Directory.render(&colours, false);
        assert_eq!(rendered.to_string(), Fixed(2).bold().paint("d").to_string());
    }

    #[test]
    fn test_filetype_render_mount_point_directory() {
        let colours = DummyColours;
        let rendered = f::Type::Directory.render(&colours, true);
        assert_eq!(rendered.to_string(), Fixed(2).bold().paint("D").to_string());
    }

    #[test]
    fn test_filetype_render_all_types() {
        let colours = DummyColours;
        assert_eq!(
            f::Type::File.render(&colours, false).to_string(),
            Fixed(1).paint(".").to_string()
        );
        assert_eq!(
            f::Type::Pipe.render(&colours, false).to_string(),
            Fixed(3).paint("|").to_string()
        );
        assert_eq!(
            f::Type::Link.render(&colours, false).to_string(),
            Fixed(4).paint("l").to_string()
        );
        assert_eq!(
            f::Type::BlockDevice.render(&colours, false).to_string(),
            Fixed(5).paint("b").to_string()
        );
        assert_eq!(
            f::Type::CharDevice.render(&colours, false).to_string(),
            Fixed(6).paint("c").to_string()
        );
        assert_eq!(
            f::Type::Socket.render(&colours, false).to_string(),
            Fixed(7).paint("s").to_string()
        );
        assert_eq!(
            f::Type::Special.render(&colours, false).to_string(),
            Fixed(8).paint("?").to_string()
        );
    }

    #[test]
    fn test_filetype_render_json() {
        assert_eq!(f::Type::File.render_json(), ".");
        assert_eq!(f::Type::Directory.render_json(), "d");
        assert_eq!(f::Type::Pipe.render_json(), "|");
        assert_eq!(f::Type::Link.render_json(), "l");
        assert_eq!(f::Type::BlockDevice.render_json(), "b");
        assert_eq!(f::Type::CharDevice.render_json(), "c");
        assert_eq!(f::Type::Socket.render_json(), "s");
        assert_eq!(f::Type::Special.render_json(), "?");
    }
}
