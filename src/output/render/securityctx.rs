// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};

impl f::SecurityContext<'_> {
    pub fn render<C: Colours>(&self, colours: &C) -> TextCell {
        match &self.context {
            f::SecurityContextType::None => TextCell::paint_str(colours.none(), "?"),
            f::SecurityContextType::SELinux(context) => {
                let mut chars = Vec::with_capacity(7);

                for (i, part) in context.split(':').enumerate() {
                    let partcolour = match i {
                        0 => colours.selinux_user(),
                        1 => colours.selinux_role(),
                        2 => colours.selinux_type(),
                        _ => colours.selinux_range(),
                    };
                    if i > 0 {
                        chars.push(colours.selinux_colon().paint(":"));
                    }
                    chars.push(partcolour.paint(String::from(part)));
                }

                TextCell {
                    contents: chars.into(),
                    width: DisplayWidth::from(context.as_ref()),
                }
            }
        }
    }

    pub fn render_json(&self) -> Option<String> {
        match &self.context {
            f::SecurityContextType::None => None,
            f::SecurityContextType::SELinux(context) => {
                let mut chars = Vec::with_capacity(7);

                for (i, part) in context.split(':').enumerate() {
                    if i > 0 {
                        chars.push(":".to_string());
                    }
                    chars.push(String::from(part));
                }

                Some(chars.join(""))
            }
        }
    }
}

#[rustfmt::skip]
pub trait Colours {
    fn none(&self) -> Style;
    fn selinux_colon(&self) -> Style;
    fn selinux_user(&self)  -> Style;
    fn selinux_role(&self)  -> Style;
    fn selinux_type(&self)  -> Style;
    fn selinux_range(&self) -> Style;
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_ansi_term::Color;
    use std::borrow::Cow;

    struct TestColours;

    impl Colours for TestColours {
        fn none(&self) -> Style {
            Color::DarkGray.normal()
        }
        fn selinux_colon(&self) -> Style {
            Color::White.normal()
        }
        fn selinux_user(&self) -> Style {
            Color::Red.bold()
        }
        fn selinux_role(&self) -> Style {
            Color::Green.normal()
        }
        fn selinux_type(&self) -> Style {
            Color::Blue.normal()
        }
        fn selinux_range(&self) -> Style {
            Color::Yellow.normal()
        }
    }

    #[test]
    fn test_none_security_context() {
        let colours = TestColours;
        let ctx = f::SecurityContext {
            context: f::SecurityContextType::None,
        };

        let cell = ctx.render(&colours);
        assert_eq!(*cell.width, 1);
        assert_eq!(ctx.render_json(), None);
    }

    #[test]
    fn test_selinux_standard_four_part_context() {
        let colours = TestColours;
        let raw = "unconfined_u:unconfined_r:unconfined_t:s0-s0:c0.c1023";
        let ctx = f::SecurityContext {
            context: f::SecurityContextType::SELinux(Cow::Borrowed(raw)),
        };

        let cell = ctx.render(&colours);
        assert_eq!(*cell.width, raw.len());

        let json = ctx.render_json();
        assert_eq!(json, Some(raw.to_string()));
    }

    #[test]
    fn test_selinux_mcs_translated_context() {
        let colours = TestColours;
        let trans = "system_u:object_r:user_home_t:CompanyConfidential".to_string();
        let ctx = f::SecurityContext {
            context: f::SecurityContextType::SELinux(Cow::Owned(trans.clone())),
        };

        let cell = ctx.render(&colours);
        assert_eq!(*cell.width, trans.len());

        let json = ctx.render_json();
        assert_eq!(json, Some(trans));
    }

    #[test]
    fn test_selinux_three_part_context() {
        let colours = TestColours;
        let raw = "system_u:object_r:default_t";
        let ctx = f::SecurityContext {
            context: f::SecurityContextType::SELinux(Cow::Borrowed(raw)),
        };

        let cell = ctx.render(&colours);
        assert_eq!(*cell.width, raw.len());
        assert_eq!(ctx.render_json(), Some(raw.to_string()));
    }

    #[test]
    fn test_selinux_single_part_context() {
        let colours = TestColours;
        let raw = "unlabeled";
        let ctx = f::SecurityContext {
            context: f::SecurityContextType::SELinux(Cow::Borrowed(raw)),
        };

        let cell = ctx.render(&colours);
        assert_eq!(*cell.width, raw.len());
        assert_eq!(ctx.render_json(), Some(raw.to_string()));
    }
}
