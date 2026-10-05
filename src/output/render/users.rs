// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;
use unicode_width::UnicodeWidthChar;
use uzers::Users;

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::output::table::UserFormat;

pub trait Render: Sized {
    fn render<C: Colours, U: Users>(self, colours: &C, users: &U, format: UserFormat) -> TextCell {
        self.render_within(colours, users, format, None)
    }

    /// Render the user, cutting a name wider than `width` columns.
    fn render_within<C: Colours, U: Users>(
        self,
        colours: &C,
        users: &U,
        format: UserFormat,
        width: Option<usize>,
    ) -> TextCell;

    fn render_json<U: Users>(self, users: &U, format: UserFormat) -> Option<String>;
}

/// `name`, cut to `width` display columns with an ellipsis in the last one
/// when it is wider. Only names are cut: a number cut short would read as
/// another number.
#[must_use]
pub fn fit_name(name: String, width: Option<usize>) -> String {
    let Some(width) = width else {
        return name;
    };
    if unicode_width::UnicodeWidthStr::width(name.as_str()) <= width {
        return name;
    }

    let mut fitted = String::with_capacity(width + 3);
    let mut used = 0;
    for c in name.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        used += w;
        fitted.push(c);
    }
    fitted.push('…');
    fitted
}

impl Render for Option<f::User> {
    fn render_within<C: Colours, U: Users>(
        self,
        colours: &C,
        users: &U,
        format: UserFormat,
        width: Option<usize>,
    ) -> TextCell {
        #[rustfmt::skip]
        let uid = match self {
            Some(u) => u.0,
            None    => return TextCell::blank(colours.no_user()),
        };
        #[rustfmt::skip]
        let user_name = match (format, users.get_user_by_uid(uid)) {
            (_, None)                      => uid.to_string(),
            (UserFormat::Numeric, _)       => uid.to_string(),
            (UserFormat::Name, Some(user)) => fit_name(user.name().to_string_lossy().into(), width),
        };

        let style = if users.get_current_uid() == uid {
            colours.you()
        } else if uid == 0 {
            colours.root()
        } else {
            colours.other()
        };
        TextCell::paint(style, user_name)
    }

    fn render_json<U: Users>(self, users: &U, format: UserFormat) -> Option<String> {
        let uid = self?.0;

        Some(match (format, users.get_user_by_uid(uid)) {
            (_, None) => uid.to_string(),
            (UserFormat::Numeric, _) => uid.to_string(),
            (UserFormat::Name, Some(user)) => user.name().to_string_lossy().into(),
        })
    }
}

pub trait Colours {
    fn you(&self) -> Style;
    fn other(&self) -> Style;
    fn root(&self) -> Style;
    fn no_user(&self) -> Style;
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::{Colours, Render, fit_name};
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;
    use crate::output::table::UserFormat;

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use uzers::User;
    use uzers::mock::MockUsers;

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn you(&self)          -> Style { Red.bold() }
        fn other(&self) -> Style { Blue.underline() }
        fn root(&self)         -> Style { Blue.underline() }
        fn no_user(&self)      -> Style { Black.italic() }
    }

    #[test]
    fn a_name_wider_than_the_width_is_cut_with_an_ellipsis() {
        assert_eq!(fit_name("enoch".into(), None), "enoch");
        assert_eq!(fit_name("enoch".into(), Some(5)), "enoch");
        assert_eq!(fit_name("enoch".into(), Some(4)), "eno…");
        assert_eq!(fit_name("enoch".into(), Some(1)), "…");
        assert_eq!(fit_name("firstname.lastname".into(), Some(8)), "firstna…");
        // Cut by columns: each of these takes two.
        assert_eq!(fit_name("山田太郎".into(), Some(8)), "山田太郎");
        assert_eq!(fit_name("山田太郎".into(), Some(6)), "山田…");
        assert_eq!(fit_name("山田太郎".into(), Some(4)), "山…");
        assert_eq!(fit_name("山田太郎".into(), Some(2)), "…");
        assert_eq!(fit_name("ünïcödé".into(), Some(4)), "ünï…");
    }

    #[test]
    fn only_names_are_cut() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "enoch", 100));

        let user = Some(f::User(1000));
        assert_eq!(
            user.render_within(&TestColours, &users, UserFormat::Name, Some(3)),
            TextCell::paint_str(Red.bold(), "en…")
        );
        assert_eq!(
            user.render_within(&TestColours, &users, UserFormat::Numeric, Some(3)),
            TextCell::paint_str(Red.bold(), "1000")
        );

        // A user with no name is shown by number, and so is not cut.
        let unnamed = Some(f::User(2000));
        assert_eq!(
            unnamed.render_within(&TestColours, &users, UserFormat::Name, Some(3)),
            TextCell::paint_str(Blue.underline(), "2000")
        );
    }

    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "enoch", 100));

        let user = Some(f::User(1000));
        let expected = TextCell::paint_str(Red.bold(), "enoch");
        #[rustfmt::skip]
        assert_eq!(expected, user.render(&TestColours, &users, UserFormat::Name));

        let expected = TextCell::paint_str(Red.bold(), "1000");
        #[rustfmt::skip]
        assert_eq!(expected, user.render(&TestColours, &users, UserFormat::Numeric));
    }

    #[test]
    fn unnamed() {
        let users = MockUsers::with_current_uid(1000);

        let user = Some(f::User(1000));
        let expected = TextCell::paint_str(Red.bold(), "1000");
        #[rustfmt::skip]
        assert_eq!(expected, user.render(&TestColours, &users, UserFormat::Name));
        #[rustfmt::skip]
        assert_eq!(expected, user.render(&TestColours, &users, UserFormat::Numeric));
    }

    #[test]
    fn different_named() {
        let mut users = MockUsers::with_current_uid(0);
        users.add_user(User::new(1000, "enoch", 100));

        let user = Some(f::User(1000));
        let expected = TextCell::paint_str(Blue.underline(), "enoch");
        assert_eq!(
            expected,
            user.render(&TestColours, &users, UserFormat::Name)
        );
    }

    #[test]
    fn different_unnamed() {
        let user = Some(f::User(1000));
        let expected = TextCell::paint_str(Blue.underline(), "1000");
        assert_eq!(
            expected,
            user.render(
                &TestColours,
                &MockUsers::with_current_uid(0),
                UserFormat::Numeric
            )
        );
    }

    #[test]
    fn overflow() {
        let user = Some(f::User(2_147_483_648));
        let expected = TextCell::paint_str(Blue.underline(), "2147483648");
        assert_eq!(
            expected,
            user.render(
                &TestColours,
                &MockUsers::with_current_uid(0),
                UserFormat::Numeric
            )
        );
    }

    #[test]
    fn named_json() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "enoch", 100));

        let user = Some(f::User(1000));
        let expected = Some("enoch".to_string());
        #[rustfmt::skip]
        assert_eq!(expected, user.render_json(&users, UserFormat::Name));

        let expected = Some("1000".to_string());
        #[rustfmt::skip]
        assert_eq!(expected, user.render_json(&users, UserFormat::Numeric));
    }

    #[test]
    fn unnamed_json() {
        let users = MockUsers::with_current_uid(1000);

        let user = Some(f::User(1000));
        let expected = Some("1000".to_string());
        #[rustfmt::skip]
        assert_eq!(expected, user.render_json(&users, UserFormat::Name));
        #[rustfmt::skip]
        assert_eq!(expected, user.render_json(&users, UserFormat::Numeric));
    }

    #[test]
    fn different_named_json() {
        let mut users = MockUsers::with_current_uid(0);
        users.add_user(User::new(1000, "enoch", 100));

        let user = Some(f::User(1000));
        let expected = Some("enoch".to_string());
        assert_eq!(expected, user.render_json(&users, UserFormat::Name));
    }

    #[test]
    fn different_unnamed_json() {
        let user = Some(f::User(1000));
        let expected = Some("1000".to_string());
        assert_eq!(
            expected,
            user.render_json(&MockUsers::with_current_uid(0), UserFormat::Numeric)
        );
    }

    #[test]
    fn overflow_json() {
        let user = Some(f::User(2_147_483_648));
        let expected = Some("2147483648".to_string());
        assert_eq!(
            expected,
            user.render_json(&MockUsers::with_current_uid(0), UserFormat::Numeric)
        );
    }
}
