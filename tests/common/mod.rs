// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(dead_code, unused_imports)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Locates the `lez` binary for running CLI integration tests.
pub fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current_exe");
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop(); // Remove deps
    }
    path.push("lez");
    path
}

/// Checks whether git is available on the system.
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Fails the calling test when git is missing.
///
/// A test that quietly returns early is reported as passed although it
/// checked nothing, so a machine without git has to show up as a failure.
pub fn require_git() {
    assert!(
        git_available(),
        "this test needs `git` on PATH; install it rather than letting the test pass without running"
    );
}

/// A configuration directory that never exists, so neither the developer's
/// own `config.toml` and `theme.yml` nor an eza one can change what a test
/// sees. `LEZ_CONFIG_DIR` outranks `XDG_CONFIG_HOME`, the platform directory
/// and `$HOME/.config`, so pointing it here switches all of them off.
pub fn no_config_dir() -> PathBuf {
    bin_path().with_file_name("lez-tests-no-config-dir")
}

/// Whether this process is subject to permission checks.
///
/// Root, or anything holding `CAP_DAC_OVERRIDE`, reads a directory whose mode
/// is `000`, so a test that expects `EACCES` has nothing to observe there. The
/// probe asks the kernel instead of guessing from the user id. CI runs as an
/// ordinary user, so a CI run that lands here is a misconfiguration and fails.
#[cfg(unix)]
pub fn permission_checks_apply() -> bool {
    use std::os::unix::fs::PermissionsExt;

    let probe = TempTestDir::new("permission_probe");
    let locked = probe.create_dir("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))
        .expect("failed to lock the probe directory");
    let denied = fs::read_dir(&locked).is_err();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))
        .expect("failed to unlock the probe directory");

    if !denied {
        assert!(
            std::env::var_os("CI").is_none(),
            "permission checks are bypassed on CI, so tests that rely on EACCES prove nothing"
        );
        eprintln!("skipped: this process bypasses permission checks (running as root?)");
    }
    denied
}

/// Creates an isolated Command for `lez` CLI integration tests.
/// - Clears all ambient environment variables to prevent host shell pollution.
/// - Passes essential environment variables (`PATH`, `HOME`, `TMPDIR`, Windows system roots).
/// - Points configuration discovery at [`no_config_dir`].
/// - Sets a neutral baseline (`TERM=dumb`).
pub fn lez_cmd() -> Command {
    let mut cmd = Command::new(bin_path());
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        cmd.env("TMPDIR", tmpdir);
    }
    // Under `cargo llvm-cov` the binary is instrumented; without this it
    // writes `default_*.profraw` into its working directory, which then
    // shows up in listings of that directory.
    if let Ok(profile) = std::env::var("LLVM_PROFILE_FILE") {
        cmd.env("LLVM_PROFILE_FILE", profile);
    }
    #[cfg(windows)]
    {
        if let Ok(val) = std::env::var("SystemRoot") {
            cmd.env("SystemRoot", val);
        }
        if let Ok(val) = std::env::var("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", val);
        }
        if let Ok(val) = std::env::var("USERPROFILE") {
            cmd.env("USERPROFILE", val);
        }
        if let Ok(val) = std::env::var("ComSpec") {
            cmd.env("ComSpec", val);
        }
    }
    cmd.env("LEZ_CONFIG_DIR", no_config_dir());
    cmd.env("TERM", "dumb");
    // Without a locale variable, lez asks the system: on macOS that is the
    // user's region (`en_US`), whose collation orders names differently.
    // `LANG` has the lowest precedence, so a test that sets `LC_ALL`,
    // `LC_COLLATE` or `LANG` itself still wins.
    cmd.env("LANG", "C");
    cmd
}

/// A whole number as lez prints it in the tests' locale. On Unix that is
/// `LANG=C`, which groups no digits. Windows has no such variable: lez
/// takes the user's regional format, so its separator is read here from
/// the registry, where Windows keeps it.
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let Some(separator) = thousands_separator() else {
        return digits;
    };
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push_str(separator);
        }
        out.push(digit);
    }
    out
}

#[cfg(not(windows))]
fn thousands_separator() -> Option<&'static str> {
    None
}

/// `sThousand` under `HKCU\Control Panel\International`, from a line
/// `reg` prints as `    sThousand    REG_SZ    ,`.
#[cfg(windows)]
fn thousands_separator() -> Option<&'static str> {
    static SEPARATOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let separator = SEPARATOR.get_or_init(|| {
        let output = Command::new("reg")
            .args([
                "query",
                r"HKCU\Control Panel\International",
                "/v",
                "sThousand",
            ])
            .output()
            .expect("run reg");
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.lines()
            .find_map(|line| line.split_once("REG_SZ"))
            .map(|(_, value)| value.trim_start_matches(' ').trim_end_matches(['\r', '\n']))
            .unwrap_or_else(|| panic!("no sThousand in:\n{text}"))
            .to_owned()
    });
    Some(separator)
}

/// `path`, written with `/`, in the host's separator. lez builds the paths
/// it prints, such as the headers of a recursive listing, with `Path::join`,
/// so on Windows they read `.\sub\deep`.
pub fn native(path: &str) -> String {
    path.replace('/', std::path::MAIN_SEPARATOR_STR)
}

/// The permissions column lez prints for the symlink at `path` itself,
/// such as `lrwxrwxrwx`: Linux gives every symlink mode 0777, while macOS
/// takes it from the umask.
#[cfg(unix)]
pub fn symlink_permissions(path: &Path) -> String {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::symlink_metadata(path)
        .expect("lstat the link")
        .permissions()
        .mode();
    let bits = (0..9).map(|i| {
        if mode & (0o400 >> i) == 0 {
            '-'
        } else {
            ['r', 'w', 'x'][i % 3]
        }
    });
    std::iter::once('l').chain(bits).collect()
}

/// An entry of a directory tree, with the entries under it, for [`draw_tree`].
pub struct TreeNode(pub String, pub Vec<TreeNode>);

impl TreeNode {
    pub fn leaf(name: impl Into<String>) -> Self {
        Self(name.into(), Vec::new())
    }
}

/// A directory tree as `lez -T .` draws it with colours off: `.`, then each
/// entry on its own line behind `├── ` or, for the last of its siblings,
/// `└── `, and the entries under it behind `│   ` or four spaces.
pub fn draw_tree(root: &[TreeNode]) -> String {
    fn walk(nodes: &[TreeNode], indent: &str, out: &mut String) {
        for (i, TreeNode(name, children)) in nodes.iter().enumerate() {
            let last = i + 1 == nodes.len();
            out.push_str(indent);
            out.push_str(if last { "└── " } else { "├── " });
            out.push_str(name);
            out.push('\n');
            let below = format!("{indent}{}", if last { "    " } else { "│   " });
            walk(children, &below, out);
        }
    }
    let mut out = String::from(".\n");
    walk(root, "", &mut out);
    out
}

/// A managed temporary directory that automatically cleans up on `Drop`.
pub struct TempTestDir {
    _temp_dir: tempfile::TempDir,
    pub path: PathBuf,
}

impl TempTestDir {
    pub fn new(prefix: &str) -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("lez_test_{prefix}_"))
            .tempdir()
            .expect("failed to create temp dir");
        let path = temp_dir.path().to_path_buf();
        Self {
            _temp_dir: temp_dir,
            path,
        }
    }

    /// A directory directly under `/tmp` with a short name, for fixtures
    /// that bind a Unix socket: a socket path may be at most 104 bytes on
    /// macOS, and its per-user temp directory alone takes about 50.
    #[cfg(unix)]
    pub fn under_tmp() -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix("lez")
            .tempdir_in("/tmp")
            .expect("failed to create temp dir under /tmp");
        let path = temp_dir.path().to_path_buf();
        Self {
            _temp_dir: temp_dir,
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_empty_file(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        StdFile::create(&p).expect("failed to create file");
        p
    }

    pub fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        let mut f = StdFile::create(&p).expect("failed to create file");
        f.write_all(content).expect("failed to write content");
        p
    }

    pub fn create_dir(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(&p).expect("failed to create dir");
        p
    }

    #[cfg(unix)]
    pub fn create_symlink(&self, target: &str, link: &str) -> PathBuf {
        use std::os::unix::fs::symlink;
        let p = self.path.join(link);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        symlink(target, &p).expect("failed to create symlink");
        p
    }

    /// Creates a rich, multi-type sample directory tree for testing
    /// (subdirectories, multiple file extensions, hidden files, empty directories, symlinks)
    /// in pure Rust without relying on platform-specific shell scripts like `dir-generator.sh`.
    pub fn create_sample_tree(&self) {
        self.create_dir("documents");
        self.create_file("documents/report.pdf", b"%PDF-1.4 dummy pdf content");
        self.create_file("documents/notes.txt", b"meeting notes");
        self.create_file(
            "documents/data.csv",
            b"id,name,score\n1,alice,100\n2,bob,95\n",
        );

        self.create_dir("src/components");
        self.create_file(
            "src/main.rs",
            b"fn main() {\n    println!(\"Hello World\");\n}\n",
        );
        self.create_file("src/lib.rs", b"pub fn helper() -> bool { true }\n");
        self.create_file(
            "src/components/button.tsx",
            b"export const Button = () => <button />;\n",
        );
        self.create_file(
            "src/components/style.css",
            b".btn { color: #fff; background: #007bff; }\n",
        );

        self.create_dir("empty_folder");
        self.create_dir("deep/level1/level2/level3");
        self.create_file(
            "deep/level1/level2/level3/leaf.bin",
            &[0x00, 0x01, 0x02, 0x03],
        );

        self.create_dir(".hidden_folder");
        self.create_file(".hidden_folder/secret.key", b"super_secret_key");
        self.create_file(".config.json", b"{\"theme\": \"dark\", \"debug\": false}\n");

        #[cfg(unix)]
        {
            let _ = self.create_symlink("documents/notes.txt", "notes_link.txt");
            let _ = self.create_symlink("documents", "docs_symlink");
            let _ = self.create_symlink("nonexistent_target", "broken_symlink");
        }
    }
}

/// Alias for backwards compatibility with tests using `TempEnv`.
pub type TempEnv = TempTestDir;

/// A managed temporary Git repository fixture.
///
/// Every git command runs with the global and system configuration switched
/// off, so a developer's `commit.gpgsign`, hooks path or default branch cannot
/// change what the repository looks like. The initial branch is always `main`.
pub struct TempGitRepo {
    _temp_dir: tempfile::TempDir,
    pub path: PathBuf,
}

impl TempGitRepo {
    /// Creates and initialises the repository, failing the test when git is
    /// missing or `git init` does not succeed.
    pub fn new(prefix: &str) -> Self {
        require_git();
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("lez_git_{prefix}_"))
            .tempdir()
            .expect("failed to create temp dir for git repo");
        let path = temp_dir.path().to_path_buf();

        let repo = Self {
            _temp_dir: temp_dir,
            path,
        };
        repo.git(&["-c", "init.defaultBranch=main", "init", "-q"]);
        repo
    }

    /// Like [`TempGitRepo::new`], but the repository is the directory `name`
    /// inside a temporary directory of its own, for listings of the parent
    /// (`--git-repos`) that must not take in the rest of the system's temp
    /// directory.
    pub fn named(prefix: &str, name: &str) -> Self {
        require_git();
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("lez_git_{prefix}_"))
            .tempdir()
            .expect("failed to create temp dir for git repo");
        let path = temp_dir.path().join(name);
        fs::create_dir(&path).expect("failed to create the repository directory");

        let repo = Self {
            _temp_dir: temp_dir,
            path,
        };
        repo.git(&["-c", "init.defaultBranch=main", "init", "-q"]);
        repo
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The directory the repository sits in: its own temporary directory for
    /// [`TempGitRepo::named`], the system temp directory otherwise.
    pub fn parent(&self) -> &Path {
        self.path.parent().expect("a repository has a parent")
    }

    pub fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        let mut f = StdFile::create(&p).expect("failed to create file");
        f.write_all(content).expect("failed to write content");
        p
    }

    pub fn write_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        self.create_file(rel, content)
    }

    #[cfg(unix)]
    pub fn create_symlink(&self, target: &str, link: &str) -> PathBuf {
        let p = self.path.join(link);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        std::os::unix::fs::symlink(target, &p).expect("failed to create symlink");
        p
    }

    /// Runs git in the repository and fails the test if it does not succeed,
    /// so a broken fixture cannot pass for the state the test meant to build.
    pub fn git(&self, args: &[&str]) {
        git_in(&self.path, args);
    }

    /// Runs git for a step that is expected to fail, such as a merge that
    /// stops on a conflict, and hands back the output to check.
    pub fn git_allow_failure(&self, args: &[&str]) -> Output {
        git_command(&self.path)
            .args(args)
            .output()
            .expect("failed to run git")
    }

    pub fn git_output(&self, args: &[&str]) -> Option<Output> {
        Some(self.git_allow_failure(args))
    }
}

/// Runs git in `dir` and fails the test if it does not succeed. For
/// repositories that are not a [`TempGitRepo`], such as several side by side
/// in one [`TempTestDir`].
#[track_caller]
pub fn git_in(dir: &Path, args: &[&str]) {
    let output = git_command(dir)
        .args(args)
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}:\n{}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A git command for `dir` that ignores the global and system configuration
/// and carries a fixed identity. Automatic maintenance is off: newer git
/// runs it in the background after a commit, where it can outlive the test
/// and hold its output open, which nextest reports as a leak.
pub fn git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c",
        "user.name=Test User",
        "-c",
        "user.email=test@example.com",
        "-c",
        "gc.auto=0",
        "-c",
        "maintenance.auto=false",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "tag.gpgsign=false",
        "-c",
        "protocol.file.allow=always",
    ])
    .current_dir(dir)
    .env("GIT_CONFIG_GLOBAL", "/dev/null")
    .env("GIT_CONFIG_SYSTEM", "/dev/null")
    .env("GIT_CONFIG_NOSYSTEM", "1")
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE")
    .env_remove("GIT_INDEX_FILE");
    cmd
}

/// Flags that reduce the long view to its name column, so a test can compare
/// the `name -> target` part of a row without dates, sizes or owners.
pub const NAME_COLUMN_ONLY: [&str; 5] = [
    "-l",
    "--no-permissions",
    "--no-filesize",
    "--no-user",
    "--no-time",
];

/// Flags that reduce the long view to the Git column and the name, so a test
/// can compare whole rows such as `-N untracked_dir`.
pub const GIT_COLUMN_ONLY: [&str; 6] = [
    "-l",
    "--git",
    "--no-permissions",
    "--no-filesize",
    "--no-user",
    "--no-time",
];

/// A `lez` command run from `dir`, with configuration discovery pointed at a
/// location that does not exist so the user's own theme cannot leak in.
pub fn lez_in(dir: &Path) -> Command {
    let mut cmd = lez_cmd();
    cmd.current_dir(dir)
        .env("LEZ_CONFIG_DIR", dir.join(".no-lez-config"));
    cmd
}

/// Runs `cmd`, returning its exit code and standard output.
pub fn exit_and_stdout(cmd: &mut Command) -> (Option<i32>, String) {
    let output = cmd.output().expect("failed to run lez");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// Runs `cmd`, requires it to succeed without writing to stderr, and returns
/// its standard output.
///
/// A test that only looks at stdout would pass while lez also printed an
/// error for an entry it could not handle, so both channels are checked.
#[track_caller]
pub fn success_stdout(cmd: &mut Command) -> String {
    let output = cmd.output().expect("failed to run lez");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{cmd:?} failed: {stderr}");
    assert!(stderr.is_empty(), "{cmd:?} wrote to stderr: {stderr}");
    String::from_utf8(output.stdout).expect("lez printed non-UTF-8 output")
}

/// Runs `cmd` with stdout and stderr on one pipe, as a terminal shows them,
/// and returns its exit code and what came through, in the order written.
pub fn interleaved(cmd: &mut Command) -> (Option<i32>, String) {
    use std::io::Read;

    let (mut reader, writer) = std::io::pipe().expect("create a pipe");
    let mut child = cmd
        .stdout(writer.try_clone().expect("clone the pipe"))
        .stderr(writer)
        .spawn()
        .expect("failed to run lez");
    // The command keeps its copies of the write end until it is set up
    // again; without this the read below would wait for them forever.
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut output = String::new();
    reader.read_to_string(&mut output).expect("read the pipe");
    let status = child.wait().expect("wait for lez");
    (status.code(), output)
}

/// Linux keeps unprivileged attributes in the `user.` namespace; macOS has
/// no namespaces.
#[cfg(target_os = "linux")]
pub const XATTR_NAME: &str = "user.field";
#[cfg(target_os = "macos")]
pub const XATTR_NAME: &str = "com.example.field";

/// Sets `XATTR_NAME` on `file`. A filesystem without user attributes skips the
/// test off CI and fails it on CI, whose temp directory supports them.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn set_xattr(file: &Path, value: &[u8]) -> bool {
    set_xattr_named(file, XATTR_NAME, value)
}

/// Sets the attribute `name` on `file`, as [`set_xattr`] does.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn set_xattr_named(file: &Path, name: &str, value: &[u8]) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(file.as_os_str().as_bytes()).expect("no NUL");
    let name = std::ffi::CString::new(name).expect("no NUL");
    // SAFETY: valid NUL-terminated strings and a buffer of the given length.
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::setxattr(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            0,
        )
    };
    // SAFETY: as above; position 0 and no options.
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::setxattr(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            0,
            0,
        )
    };
    if result == 0 {
        return true;
    }
    let error = std::io::Error::last_os_error();
    assert!(
        std::env::var_os("CI").is_none() && error.raw_os_error() == Some(libc::ENOTSUP),
        "setxattr failed: {error}"
    );
    eprintln!("skipped: the temp directory's filesystem has no user attributes");
    false
}

/// Grants `caps` to `file` with `setcap`, directly as root or through a
/// password-free `sudo` as on CI. Off CI, an account that can do neither
/// skips the test, saying so; on CI it is a failure.
#[cfg(target_os = "linux")]
pub fn grant_capabilities(file: &Path, caps: &str) -> bool {
    let direct = Command::new("setcap").arg(caps).arg(file).output();
    if direct.is_ok_and(|output| output.status.success()) {
        return true;
    }
    let sudo = Command::new("sudo")
        .args(["-n", "setcap", caps])
        .arg(file)
        .output();
    if sudo.is_ok_and(|output| output.status.success()) {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "setcap should be available on CI; without it this test proves nothing"
    );
    eprintln!("skipped: setcap is not available to this account");
    false
}

/// A name from `getpwuid_r` or `getgrgid_r`, or the number when there is
/// none, as lez falls back to.
#[cfg(unix)]
pub fn owner_name(id: u32, user: bool) -> String {
    use std::ffi::CStr;

    let mut buffer = vec![0u8; 16 * 1024];
    let buffer_ptr = buffer.as_mut_ptr().cast();
    // SAFETY: each call fills a zeroed record whose strings point into
    // `buffer`, which outlives every use of them below.
    unsafe {
        if user {
            let mut record: libc::passwd = std::mem::zeroed();
            let mut found = std::ptr::null_mut();
            libc::getpwuid_r(id, &mut record, buffer_ptr, buffer.len(), &mut found);
            if !found.is_null() {
                return CStr::from_ptr(record.pw_name)
                    .to_string_lossy()
                    .into_owned();
            }
        } else {
            let mut record: libc::group = std::mem::zeroed();
            let mut found = std::ptr::null_mut();
            libc::getgrgid_r(id, &mut record, buffer_ptr, buffer.len(), &mut found);
            if !found.is_null() {
                return CStr::from_ptr(record.gr_name)
                    .to_string_lossy()
                    .into_owned();
            }
        }
    }
    id.to_string()
}

/// A zip archive of stored regular files, in the order given. The checksums
/// are left at zero: lez reads only the central directory.
pub fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, content) in entries {
        let offset = u32::try_from(out.len()).unwrap();
        let size = u32::try_from(content.len()).unwrap();
        let name_len = u16::try_from(name.len()).unwrap();
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0]);
        out.extend_from_slice(&[0; 10]);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(content);

        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
        central.extend_from_slice(&[0; 10]);
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = u32::try_from(out.len()).unwrap();
    let count = u16::try_from(entries.len()).unwrap();
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&u32::try_from(central.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}
