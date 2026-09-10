// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use crate::common::TempTestDir;
use std::fs;
use std::process::Command;

#[test]
fn test_m1_child_git_repo_gitignore_respected_under_parent_dir() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("child_git_ignore");

    // Parent directory is NOT a git repo
    let parent = temp.create_dir("workspace");

    // Inside parent, create a child git repository
    let child_repo_dir = parent.join("child_repo");
    fs::create_dir_all(&child_repo_dir).unwrap();
    let repo = git2::Repository::init(&child_repo_dir).unwrap();
    let sig = git2::Signature::now("Tester", "tester@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }

    // Create files in child repo: one tracked/normal, one ignored via .gitignore
    fs::write(child_repo_dir.join(".gitignore"), "*.secret\nbuild/\n").unwrap();
    fs::write(child_repo_dir.join("visible.txt"), "hello").unwrap();
    fs::write(child_repo_dir.join("password.secret"), "supersecret").unwrap();

    let build_dir = child_repo_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();
    fs::write(build_dir.join("output.bin"), "binary").unwrap();

    // Commit .gitignore and visible.txt
    {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add files", &tree, &[&head])
            .unwrap();
    }

    // Run lez --tree --git-ignore on parent
    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", parent.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // visible.txt MUST be present
    assert!(
        stdout.contains("visible.txt"),
        "visible.txt should appear in output: {stdout}"
    );

    // password.secret and build/ MUST be ignored
    assert!(
        !stdout.contains("password.secret"),
        "password.secret was NOT ignored by child .gitignore! Output: {stdout}"
    );
    assert!(
        !stdout.contains("output.bin"),
        "build/output.bin was NOT ignored by child .gitignore! Output: {stdout}"
    );
}

#[test]
fn test_m1_multiple_sibling_git_repos_under_common_parent() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("sibling_repos");

    // Parent dir containing two separate repos: repo_a and repo_b
    let repo_a_dir = temp.create_dir("repo_a");
    let repo_b_dir = temp.create_dir("repo_b");

    let _ = git2::Repository::init(&repo_a_dir).unwrap();
    let _ = git2::Repository::init(&repo_b_dir).unwrap();

    // Repo A ignores *.log
    fs::write(repo_a_dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(repo_a_dir.join("app.rs"), "fn main() {}").unwrap();
    fs::write(repo_a_dir.join("debug.log"), "log a").unwrap();

    // Repo B ignores *.tmp
    fs::write(repo_b_dir.join(".gitignore"), "*.tmp\n").unwrap();
    fs::write(repo_b_dir.join("lib.rs"), "pub fn test() {}").unwrap();
    fs::write(repo_b_dir.join("cache.tmp"), "cache b").unwrap();
    // Repo B does NOT ignore debug.log
    fs::write(repo_b_dir.join("debug.log"), "log b").unwrap();

    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("app.rs"));
    assert!(stdout.contains("lib.rs"));
    // cache.tmp in repo_b must be ignored
    assert!(!stdout.contains("cache.tmp"));
    // debug.log in repo_a should be ignored, but debug.log in repo_b should be visible
    assert!(
        stdout.contains("debug.log"),
        "debug.log in repo_b should be visible"
    );
}

#[test]
fn test_m1_submodule_dot_git_file_handled() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("submod_file");

    // Simulate submodule where .git is a file
    let submod_dir = temp.create_dir("parent/submodule");
    let git_dir_target = temp.create_dir("git_modules_target");
    let _ = git2::Repository::init(&git_dir_target).unwrap();

    fs::write(
        submod_dir.join(".git"),
        format!("gitdir: {}\n", git_dir_target.display()),
    )
    .unwrap();
    fs::write(submod_dir.join(".gitignore"), "*.submod_ignored\n").unwrap();
    fs::write(submod_dir.join("kept.txt"), "ok").unwrap();
    fs::write(submod_dir.join("trash.submod_ignored"), "bad").unwrap();

    let output = Command::new(bin_path)
        .args([
            "--recurse",
            "--git-ignore",
            "-a",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kept.txt"));
}

#[test]
fn test_child_git_repo_status_in_tree_and_recurse_without_git_ignore() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("child_repo_status");

    // temp is a non-repo directory. child_repo is a Git repository.
    let child_repo_dir = temp.create_dir("child_repo");
    let repo = git2::Repository::init(&child_repo_dir).unwrap();
    let sig = git2::Signature::now("Tester", "test@test.com").unwrap();

    let tracked_path = child_repo_dir.join("tracked.txt");
    fs::write(&tracked_path, "initial content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init commit", &tree, &[])
        .unwrap();

    // Modify tracked.txt (unstaged modified => -M)
    fs::write(&tracked_path, "modified content").unwrap();
    // Create new_file.txt (untracked/new => -N)
    let new_path = child_repo_dir.join("new_file.txt");
    fs::write(&new_path, "new content").unwrap();

    // Run lez -l --git --header --tree on temp WITHOUT --git-ignore
    let output_tree = Command::new(bin_path)
        .args([
            "-l",
            "--git",
            "--header",
            "--tree",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez");
    assert!(output_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&output_tree.stdout);

    // 1. Header must contain Git column header
    assert!(
        stdout_tree.contains("Git"),
        "Tree mode on parent non-repo path must preserve Git column header: {stdout_tree}"
    );
    // 2. Output must show modified and new status for child repo files
    assert!(
        stdout_tree.contains("-M") || stdout_tree.contains("M"),
        "Tree mode must show modified status for tracked.txt in child repo: {stdout_tree}"
    );
    assert!(
        stdout_tree.contains("-N") || stdout_tree.contains("N"),
        "Tree mode must show new status for new_file.txt in child repo: {stdout_tree}"
    );

    // Run lez -l --git --header --recurse on temp WITHOUT --git-ignore
    let output_recurse = Command::new(bin_path)
        .args([
            "-l",
            "--git",
            "--header",
            "--recurse",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez");
    assert!(output_recurse.status.success());
    let stdout_recurse = String::from_utf8_lossy(&output_recurse.stdout);

    assert!(
        stdout_recurse.contains("-M") || stdout_recurse.contains("M"),
        "Recurse mode must show modified status for child repo: {stdout_recurse}"
    );
}

#[test]
fn test_child_submodule_dot_git_file_status_in_tree_without_git_ignore() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("child_submod_status");

    let parent = temp.create_dir("workspace");
    let dep_dir = temp.create_dir("dep");
    let dep_repo = git2::Repository::init(&dep_dir).unwrap();
    let sig = git2::Signature::now("Tester", "test@test.com").unwrap();
    let dep_file = dep_dir.join("file.txt");
    fs::write(&dep_file, "dep initial").unwrap();
    {
        let mut idx = dep_repo.index().unwrap();
        idx.add_path(std::path::Path::new("file.txt")).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = dep_repo.find_tree(tree_id).unwrap();
        dep_repo
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }

    let sub_repo_dir = parent.join("sub_repo");
    let sub_repo = git2::Repository::init(&sub_repo_dir).unwrap();
    let mut sub = sub_repo
        .submodule(
            dep_dir.to_str().unwrap(),
            std::path::Path::new("mysub"),
            true,
        )
        .unwrap();
    sub.clone(None).unwrap();
    sub.add_finalize().unwrap();

    let sub_file = sub_repo_dir.join("mysub").join("file.txt");
    fs::write(&sub_file, "dep modified").unwrap();

    let output = Command::new(bin_path)
        .args([
            "-l",
            "--git",
            "--header",
            "--tree",
            parent.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute lez");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Git"),
        "Tree mode on parent must show Git column header: {stdout}"
    );
    assert!(
        stdout.contains("-M") || stdout.contains("M"),
        "Tree mode must show modified status for modified submodule file: {stdout}"
    );
}

#[test]
fn test_m1_deeply_nested_git_repo_traversal() {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    let temp = TempTestDir::new("deep_nested_repo");

    let deep_repo_dir = temp.create_dir("l1/l2/l3/l4/deep_repo");
    let _repo = git2::Repository::init(&deep_repo_dir).unwrap();

    fs::write(deep_repo_dir.join(".gitignore"), "*.secret\n").unwrap();
    fs::write(deep_repo_dir.join("real_code.rs"), "fn main() {}").unwrap();
    fs::write(deep_repo_dir.join("token.secret"), "12345").unwrap();

    let output = Command::new(bin_path)
        .args(["--tree", "--git-ignore", "-a", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to execute lez binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("real_code.rs"));
    assert!(!stdout.contains("token.secret"));
}
