// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Repositories below the listed directory rather than above it: lez has to
//! find each one while it descends, apply that repository's own
//! `.gitignore`, and show statuses relative to it.

use std::path::Path;

use crate::common::{GIT_COLUMN_ONLY, TempTestDir, git_in, lez_in, native, success_stdout};

fn repo(dir: &TempTestDir, rel: &str, files: &[(&str, &str)]) {
    let path = dir.create_dir(rel);
    git_in(&path, &["-c", "init.defaultBranch=main", "init", "-q"]);
    for (name, content) in files {
        dir.create_file(&format!("{rel}/{name}"), content.as_bytes());
    }
}

fn commit_all(path: &Path) {
    git_in(path, &["add", "."]);
    git_in(path, &["commit", "-q", "-m", "fixture"]);
}

fn lez(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args))
}

#[test]
fn a_child_repositorys_gitignore_applies_below_a_plain_parent() {
    let dir = TempTestDir::new("child_ignore");
    repo(
        &dir,
        "workspace/child_repo",
        &[
            (".gitignore", "*.secret\nbuild/\n"),
            ("visible.txt", "hello"),
            ("password.secret", "secret"),
            ("build/output.bin", "binary"),
        ],
    );
    commit_all(&dir.path().join("workspace/child_repo"));

    assert_eq!(
        lez(dir.path(), &["--tree", "--git-ignore", "-a", "workspace"]),
        "workspace\n└── child_repo\n    ├── .gitignore\n    └── visible.txt\n"
    );
}

/// Each repository's patterns stay inside it: `debug.log` is ignored in
/// `repo_a` and kept in `repo_b`, a level deeper too.
#[test]
fn sibling_repositories_keep_their_own_ignore_rules() {
    let dir = TempTestDir::new("sibling_repos");
    repo(
        &dir,
        "repo_a",
        &[
            (".gitignore", "*.log\n"),
            ("app.rs", "x"),
            ("debug.log", "a"),
        ],
    );
    repo(
        &dir,
        "repo_b",
        &[
            (".gitignore", "*.tmp\n"),
            ("lib.rs", "x"),
            ("cache.tmp", "b"),
            ("debug.log", "b"),
        ],
    );
    repo(
        &dir,
        "l1/l2/l3/deep_repo",
        &[
            (".gitignore", "*.secret\n"),
            ("real_code.rs", "x"),
            ("token.secret", "s"),
        ],
    );

    assert_eq!(
        lez(dir.path(), &["--tree", "--git-ignore", "-a"]),
        ".\n\
         ├── l1\n\
         │   └── l2\n\
         │       └── l3\n\
         │           └── deep_repo\n\
         │               ├── .gitignore\n\
         │               └── real_code.rs\n\
         ├── repo_a\n\
         │   ├── .gitignore\n\
         │   └── app.rs\n\
         └── repo_b\n    \
             ├── .gitignore\n    \
             ├── debug.log\n    \
             └── lib.rs\n"
    );
}

/// A directory's status combines those of everything in it, and a new
/// file outranks a modified one, so `child_repo` reads `-N`.
#[test]
fn statuses_come_from_a_child_repository_in_trees_and_recursion() {
    let dir = TempTestDir::new("child_status");
    repo(&dir, "child_repo", &[("tracked.txt", "initial")]);
    let child = dir.path().join("child_repo");
    commit_all(&child);
    dir.create_file("child_repo/tracked.txt", b"modified");
    dir.create_file("child_repo/new_file.txt", b"new");

    let mut header = GIT_COLUMN_ONLY.to_vec();
    header.extend(["--header", "-T"]);
    assert_eq!(
        lez(dir.path(), &header),
        "Git Name\n \
         -- .\n \
         -N └── child_repo\n \
         -N     ├── new_file.txt\n \
         -M     └── tracked.txt\n"
    );

    let mut recurse = GIT_COLUMN_ONLY.to_vec();
    recurse.push("-R");
    assert_eq!(
        lez(dir.path(), &recurse),
        native("-N child_repo\n\n./child_repo:\n-N new_file.txt\n-M tracked.txt\n")
    );
}

/// A submodule's `.git` is a file pointing into the parent's `.git/modules`.
/// Its files take their status, and their ignore rules, from the submodule.
#[test]
fn a_submodule_is_its_own_repository() {
    let dir = TempTestDir::new("submodule_repo");
    repo(
        &dir,
        "dep",
        &[("file.txt", "dep"), (".gitignore", "*.submod_ignored\n")],
    );
    commit_all(&dir.path().join("dep"));
    repo(&dir, "parent", &[]);
    let parent = dir.path().join("parent");
    let dep = dir.path().join("dep");
    git_in(
        &parent,
        &[
            "submodule",
            "add",
            "-q",
            dep.to_str().expect("UTF-8 path"),
            "sub",
        ],
    );
    git_in(&parent, &["commit", "-q", "-m", "add submodule"]);
    assert!(parent.join("sub/.git").is_file());
    dir.create_file("parent/sub/trash.submod_ignored", b"bad");
    dir.create_file("parent/sub/kept.txt", b"ok");
    dir.create_file("parent/sub/file.txt", b"modified");

    assert_eq!(
        lez(dir.path(), &["-R", "--git-ignore", "-a", "parent"]),
        native(".gitmodules\nsub\n\nparent/sub:\n.gitignore\nfile.txt\nkept.txt\n")
    );
    let mut tree = GIT_COLUMN_ONLY.to_vec();
    tree.extend(["-T", "parent"]);
    assert_eq!(
        lez(dir.path(), &tree),
        "-M parent\n\
         -N └── sub\n\
         -M     ├── file.txt\n\
         -N     ├── kept.txt\n\
         -I     └── trash.submod_ignored\n"
    );
}
