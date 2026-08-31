// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::output::render::FiletypeColours;

use super::{PermissionsColours as Colours, PermissionsPlusRender};

use nu_ansi_term::{AnsiString as ANSIString, Style};

impl PermissionsPlusRender for Option<f::PermissionsPlus> {
    fn render<C: Colours + FiletypeColours>(&self, colours: &C) -> TextCell {
        match self {
            Some(p) => {
                let mut chars = vec![p.attributes.render_type(colours)];
                chars.extend(p.attributes.render(colours));

                TextCell {
                    width: DisplayWidth::from(chars.len()),
                    contents: chars.into(),
                }
            }
            None => TextCell {
                width: DisplayWidth::from(0),
                contents: vec![].into(),
            },
        }
    }

    fn render_json(&self) -> Option<String> {
        self.map(|p| {
            let mut chars = vec![p.attributes.render_type_json()];
            chars.extend(p.attributes.render_json());

            chars.join("")
        })
    }
}

impl f::Attributes {
    pub fn render<C: Colours + FiletypeColours>(self, colours: &C) -> Vec<ANSIString<'static>> {
        let bit = |bit, chr: &'static str, style: Style| {
            if bit {
                style.paint(chr)
            } else {
                colours.dash().paint("-")
            }
        };

        vec![
            bit(self.archive, "a", colours.normal()),
            bit(self.readonly, "r", colours.user_read()),
            bit(self.hidden, "h", colours.special_user_file()),
            bit(self.system, "s", colours.special_other()),
        ]
    }

    pub fn render_json(self) -> Vec<&'static str> {
        let bit = |bit, chr: &'static str| {
            if bit { chr } else { "-" }
        };

        vec![
            bit(self.archive, "a"),
            bit(self.readonly, "r"),
            bit(self.hidden, "h"),
            bit(self.system, "s"),
        ]
    }

    pub fn render_type<C: Colours + FiletypeColours>(self, colours: &C) -> ANSIString<'static> {
        if self.reparse_point {
            return colours.pipe().paint("l");
        } else if self.directory {
            return colours.directory().paint("d");
        }
        colours.dash().paint("-")
    }

    pub fn render_type_json(self) -> &'static str {
        if self.reparse_point {
            return "l";
        } else if self.directory {
            return "d";
        }
        "-"
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_ansi_term::Color::*;

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn dash(&self)                -> Style { Fixed(11).normal() }
        fn user_read(&self)           -> Style { Fixed(101).normal() }
        fn user_write(&self)          -> Style { Fixed(102).normal() }
        fn user_execute_file(&self)   -> Style { Fixed(103).normal() }
        fn user_execute_other(&self)  -> Style { Fixed(113).normal() }
        fn group_read(&self)          -> Style { Fixed(104).normal() }
        fn group_write(&self)         -> Style { Fixed(105).normal() }
        fn group_execute(&self)       -> Style { Fixed(106).normal() }
        fn other_read(&self)          -> Style { Fixed(107).normal() }
        fn other_write(&self)         -> Style { Fixed(108).normal() }
        fn other_execute(&self)       -> Style { Fixed(109).normal() }
        fn special_user_file(&self)   -> Style { Fixed(110).normal() }
        fn special_other(&self)       -> Style { Fixed(111).normal() }
        fn attribute(&self)           -> Style { Fixed(112).normal() }
    }

    #[rustfmt::skip]
    impl FiletypeColours for TestColours {
        fn normal(&self)       -> Style { Fixed(1).normal() }
        fn directory(&self)    -> Style { Fixed(2).bold() }
        fn pipe(&self)         -> Style { Fixed(3).normal() }
        fn symlink(&self) -> crate::theme::LinkStyle {
            crate::theme::LinkStyle::AnsiStyle(Fixed(4).normal())
        }
        fn block_device(&self) -> Style { Fixed(5).normal() }
        fn char_device(&self)  -> Style { Fixed(6).normal() }
        fn socket(&self)       -> Style { Fixed(7).normal() }
        fn special(&self)      -> Style { Fixed(8).normal() }
        fn tag(&self, _tag: &f::TagColor) -> Style { Fixed(9).normal() }
    }

    #[test]
    fn test_none_permissions_plus() {
        let p: Option<f::PermissionsPlus> = None;
        let cell = p.render(&TestColours);
        assert_eq!(*cell.width, 0);
        assert_eq!(p.render_json(), None);
    }

    #[test]
    fn test_attributes_render_json_permutations() {
        let empty_attr = f::Attributes {
            archive: false,
            readonly: false,
            hidden: false,
            system: false,
            reparse_point: false,
            directory: false,
        };
        assert_eq!(empty_attr.render_json(), vec!["-", "-", "-", "-"]);
        assert_eq!(empty_attr.render_type_json(), "-");

        let full_attr = f::Attributes {
            archive: true,
            readonly: true,
            hidden: true,
            system: true,
            reparse_point: false,
            directory: false,
        };
        assert_eq!(full_attr.render_json(), vec!["a", "r", "h", "s"]);

        let dir_attr = f::Attributes {
            archive: false,
            readonly: true,
            hidden: false,
            system: false,
            reparse_point: false,
            directory: true,
        };
        assert_eq!(dir_attr.render_json(), vec!["-", "r", "-", "-"]);
        assert_eq!(dir_attr.render_type_json(), "d");

        let link_attr = f::Attributes {
            archive: false,
            readonly: false,
            hidden: false,
            system: false,
            reparse_point: true,
            directory: false,
        };
        assert_eq!(link_attr.render_type_json(), "l");
    }

    #[test]
    fn test_permissions_plus_render_combined() {
        let attr = f::Attributes {
            archive: true,
            readonly: false,
            hidden: true,
            system: false,
            reparse_point: false,
            directory: true,
        };
        let p = Some(f::PermissionsPlus {
            file_type: f::Type::Directory,
            attributes: attr,
            xattrs: false,
            mount: false,
        });

        let cell = p.render(&TestColours);
        assert_eq!(*cell.width, 5);
        assert_eq!(p.render_json(), Some("da-h-".to_string()));
    }
}
