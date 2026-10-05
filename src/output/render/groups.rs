// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;
use uzers::{Groups, Users};

use crate::fs::fields as f;
use crate::fs::fields::User;
use crate::output::cell::TextCell;
use crate::output::render::users::fit_name;
use crate::output::table::{GroupFormat, UserFormat};

pub trait Render: Sized {
    fn render<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        users: &U,
        user_format: UserFormat,
        group_format: GroupFormat,
        file_user: Option<User>,
    ) -> TextCell {
        self.render_within(colours, users, user_format, group_format, file_user, None)
    }

    /// Render the group, cutting a name wider than `width` columns.
    fn render_within<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        users: &U,
        user_format: UserFormat,
        group_format: GroupFormat,
        file_user: Option<User>,
        width: Option<usize>,
    ) -> TextCell;

    fn render_json<U: Groups>(self, users: &U, user_format: UserFormat) -> Option<String>;
}

impl Render for Option<f::Group> {
    fn render_within<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        users: &U,
        user_format: UserFormat,
        group_format: GroupFormat,
        file_user: Option<User>,
        width: Option<usize>,
    ) -> TextCell {
        use uzers::os::unix::GroupExt;

        let mut style = colours.not_yours();

        let gid = match self {
            Some(g) => g.0,
            None => return TextCell::blank(colours.no_group()),
        };
        let maybe_group = users.get_group_by_gid(gid);

        let current_uid = users.get_current_uid();
        if let Some(current_user) = users.get_user_by_uid(current_uid)
            && (current_user.primary_group_id() == gid
                || maybe_group
                    .as_ref()
                    .is_some_and(|g| g.members().iter().any(|u| u == current_user.name())))
        {
            style = colours.yours();
        }

        if gid == 0 && style != colours.yours() {
            style = colours.root_group();
        }

        let mut group_name = match user_format {
            UserFormat::Name => match &maybe_group {
                Some(group) => group.name().to_string_lossy().into(),
                None => gid.to_string(),
            },
            UserFormat::Numeric => gid.to_string(),
        };

        if let GroupFormat::Smart = group_format
            && let Some(file_uid) = file_user
        {
            let is_match = match user_format {
                UserFormat::Name => {
                    let user_name = users
                        .get_user_by_uid(file_uid.0)
                        .map(|u| u.name().to_string_lossy().into_owned())
                        .unwrap_or_else(|| file_uid.0.to_string());
                    let grp_name = match &maybe_group {
                        Some(group) => group.name().to_string_lossy(),
                        None => std::borrow::Cow::Owned(gid.to_string()),
                    };
                    user_name == grp_name
                }
                UserFormat::Numeric => file_uid.0 == gid,
            };
            if is_match {
                return TextCell::paint(style, ":".to_string());
            }
        }

        // The smart group above compares the whole names; only what is
        // shown is cut.
        if user_format == UserFormat::Name && maybe_group.is_some() {
            group_name = fit_name(group_name, width);
        }
        TextCell::paint(style, group_name)
    }

    fn render_json<U: Groups>(self, users: &U, user_format: UserFormat) -> Option<String> {
        let gid = self?.0;

        Some(match user_format {
            UserFormat::Numeric => gid.to_string(),
            UserFormat::Name => match users.get_group_by_gid(gid) {
                Some(group) => group.name().to_string_lossy().into(),
                None => gid.to_string(),
            },
        })
    }
}

pub trait Colours {
    fn yours(&self) -> Style;
    fn not_yours(&self) -> Style;
    fn no_group(&self) -> Style;
    fn root_group(&self) -> Style;
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::{Colours, Render};
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;
    use crate::output::table::{GroupFormat, UserFormat};

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use uzers::mock::MockUsers;
    use uzers::os::unix::GroupExt;
    use uzers::{Group, User};

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn yours(&self)     -> Style { Fixed(80).normal() }
        fn not_yours(&self) -> Style { Fixed(81).normal() }
        fn no_group(&self)   -> Style { Black.italic() }
        fn root_group(&self) -> Style { Fixed(82).normal() }
    }

    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let file_user = Some(f::User(1000));
        let expected = TextCell::paint_str(TestColours.not_yours(), "folk");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Regular,
                file_user
            )
        );

        let expected = TextCell::paint_str(TestColours.not_yours(), "100");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Regular,
                file_user
            )
        );
    }

    #[test]
    fn unnamed() {
        let users = MockUsers::with_current_uid(1000);

        let group = Some(f::Group(100));
        let file_user = Some(f::User(1000));
        let expected = TextCell::paint_str(TestColours.not_yours(), "100");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Regular,
                file_user
            )
        );
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Regular,
                file_user
            )
        );
    }

    #[test]
    fn primary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 100));
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let file_user = Some(f::User(2));
        let expected = TextCell::paint_str(TestColours.yours(), "folk");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Regular,
                file_user
            )
        );
    }

    #[test]
    fn secondary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 666));

        let test_group = Group::new(100, "folk").add_member("eve");
        users.add_group(test_group);

        let group = Some(f::Group(100));
        let file_user = Some(f::User(2));
        let expected = TextCell::paint_str(TestColours.yours(), "folk");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Regular,
                file_user
            )
        );
    }

    #[test]
    fn overflow() {
        let group = Some(f::Group(2_147_483_648));
        let file_user = Some(f::User(1000));
        let expected = TextCell::paint_str(TestColours.not_yours(), "2147483648");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &MockUsers::with_current_uid(0),
                UserFormat::Numeric,
                GroupFormat::Regular,
                file_user
            )
        );
    }

    #[test]
    fn a_group_name_is_cut_after_smart_group_compares_it_whole() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "developers", 100));
        users.add_user(User::new(1001, "develop", 100));
        users.add_group(Group::new(100, "developers"));

        let group = Some(f::Group(100));
        let render = |owner: u32, format: GroupFormat| {
            group.render_within(
                &TestColours,
                &users,
                UserFormat::Name,
                format,
                Some(f::User(owner)),
                Some(5),
            )
        };
        assert_eq!(
            render(1000, GroupFormat::Regular),
            TextCell::paint_str(TestColours.yours(), "deve…")
        );
        // The names match whole, so the group is elided even though cutting
        // would make `develop` and `developers` alike...
        assert_eq!(
            render(1000, GroupFormat::Smart),
            TextCell::paint_str(TestColours.yours(), ":")
        );
        // ...and they differ whole, so it is not.
        assert_eq!(
            render(1001, GroupFormat::Smart),
            TextCell::paint_str(TestColours.yours(), "deve…")
        );

        // A number is not cut.
        assert_eq!(
            group.render_within(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Regular,
                None,
                Some(2)
            ),
            TextCell::paint_str(TestColours.yours(), "100")
        );
    }

    #[test]
    fn smart() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "user", 100));
        users.add_user(User::new(1001, "http", 101));
        users.add_group(Group::new(100, "user"));
        users.add_group(Group::new(101, "http"));

        let user_group = Some(f::Group(100));
        let user_file = Some(f::User(1000));
        let expected = TextCell::paint_str(TestColours.yours(), ":");
        assert_eq!(
            expected,
            user_group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Smart,
                user_file
            )
        );

        let expected = TextCell::paint_str(TestColours.yours(), "100");
        assert_eq!(
            expected,
            user_group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Smart,
                user_file
            )
        );

        let same_id_file = Some(f::User(100));
        let expected = TextCell::paint_str(TestColours.yours(), ":");
        assert_eq!(
            expected,
            user_group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Smart,
                same_id_file
            )
        );

        let http_group = Some(f::Group(101));
        let expected = TextCell::paint_str(TestColours.not_yours(), "http");
        assert_eq!(
            expected,
            http_group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Smart,
                user_file
            )
        );

        let http_file = Some(f::User(1001));
        let expected = TextCell::paint_str(TestColours.not_yours(), ":");
        assert_eq!(
            expected,
            http_group.render(
                &TestColours,
                &users,
                UserFormat::Name,
                GroupFormat::Smart,
                http_file
            )
        );
    }

    #[test]
    fn named_json() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let expected = Some("folk".to_string());
        assert_eq!(expected, group.render_json(&users, UserFormat::Name));

        let expected = Some("100".to_string());
        assert_eq!(expected, group.render_json(&users, UserFormat::Numeric));
    }

    #[test]
    fn unnamed_json() {
        let users = MockUsers::with_current_uid(1000);

        let group = Some(f::Group(100));
        let expected = Some("100".to_string());
        assert_eq!(expected, group.render_json(&users, UserFormat::Name));
        assert_eq!(expected, group.render_json(&users, UserFormat::Numeric));
    }

    #[test]
    fn primary_json() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 100));
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let expected = Some("folk".to_string());
        assert_eq!(expected, group.render_json(&users, UserFormat::Name));
    }

    #[test]
    fn secondary_json() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 666));

        let test_group = Group::new(100, "folk").add_member("eve");
        users.add_group(test_group);

        let group = Some(f::Group(100));
        let expected = Some("folk".to_string());
        assert_eq!(expected, group.render_json(&users, UserFormat::Name));
    }

    #[test]
    fn overflow_json() {
        let group = Some(f::Group(2_147_483_648));
        let expected = Some("2147483648".to_string());
        assert_eq!(
            expected,
            group.render_json(&MockUsers::with_current_uid(0), UserFormat::Numeric)
        );
    }

    #[test]
    fn smart_json() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "user", 100));
        users.add_user(User::new(1001, "http", 101));
        users.add_group(Group::new(100, "user"));
        users.add_group(Group::new(101, "http"));

        let user_group = Some(f::Group(100));
        // Structured JSON output must always emit the true group name or numeric GID, never ":"
        assert_eq!(
            Some("user".to_string()),
            user_group.render_json(&users, UserFormat::Name)
        );

        assert_eq!(
            Some("100".to_string()),
            user_group.render_json(&users, UserFormat::Numeric)
        );

        let http_group = Some(f::Group(101));
        assert_eq!(
            Some("http".to_string()),
            http_group.render_json(&users, UserFormat::Name)
        );

        assert_eq!(
            Some("101".to_string()),
            http_group.render_json(&users, UserFormat::Numeric)
        );
    }

    #[test]
    fn none_group_json() {
        let users = MockUsers::with_current_uid(1000);
        let none_group: Option<f::Group> = None;
        assert_eq!(None, none_group.render_json(&users, UserFormat::Name));
        assert_eq!(None, none_group.render_json(&users, UserFormat::Numeric));
    }

    #[test]
    fn unmapped_group_smart_and_styling() {
        let mut users = MockUsers::with_current_uid(1000);
        // User with primary_group_id = 9999, but group 9999 is unmapped in /etc/group
        users.add_user(User::new(1000, "user", 9999));

        let unmapped_group = Some(f::Group(9999));

        // 1. Primary group styling must be preserved even when GID is unmapped in /etc/group
        let rendered_name = unmapped_group.render(
            &TestColours,
            &users,
            UserFormat::Name,
            GroupFormat::Regular,
            None,
        );
        assert_eq!(
            TextCell::paint_str(TestColours.yours(), "9999"),
            rendered_name
        );

        // 2. Numeric mode + smart group collapses to ":" when file UID == GID for unmapped group
        let same_id_file = Some(f::User(9999));
        let rendered_smart_numeric = unmapped_group.render(
            &TestColours,
            &users,
            UserFormat::Numeric,
            GroupFormat::Smart,
            same_id_file,
        );
        assert_eq!(
            TextCell::paint_str(TestColours.yours(), ":"),
            rendered_smart_numeric
        );

        // 3. Unmapped GID 0 receives root_group styling when not owned by current user
        let other_users = MockUsers::with_current_uid(2000);
        let unmapped_root_group = Some(f::Group(0));
        let rendered_root = unmapped_root_group.render(
            &TestColours,
            &other_users,
            UserFormat::Numeric,
            GroupFormat::Regular,
            None,
        );
        assert_eq!(
            TextCell::paint_str(TestColours.root_group(), "0"),
            rendered_root
        );

        // 4. Name mode + smart group collapses to ":" when unmapped UID == unmapped GID
        let rendered_smart_name = unmapped_group.render(
            &TestColours,
            &users,
            UserFormat::Name,
            GroupFormat::Smart,
            same_id_file,
        );
        assert_eq!(
            TextCell::paint_str(TestColours.yours(), ":"),
            rendered_smart_name
        );
    }
}
