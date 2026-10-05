<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# Upstream Triage & Compatibility Reference

> **Lineage**: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).
> `lez` ports unmerged `eza-community/eza` work by hand; it does not synchronize directly with upstream.

---

## 1. Upstream Triage Overview

**Last refreshed 2026-10-05**, against 173 open upstream PRs and 291 open upstream issues. The full sweep behind most of this file was taken on 2026-08-24 and 2026-08-25, against 149 open PRs and 286 open issues.

**Upstream has not moved since the fork.** `lez` branched from upstream `main` at `471bfbc7` (eza v0.23.5, 2026-07-09). As of 2026-10-05, upstream `main` has no commit after it, there is no release after v0.23.5, no PR has been merged since 2026-08-20, and no maintainer has commented since then. Everything upstream has merged is already in `lez`; what is left to consider is open PRs and issues.

- **PRs (173 open)**: 42 were declined in the 2026-08-24 sweep (§2). 106 had already been ported by then, and [#1924](https://github.com/eza-community/eza/pull/1924) was ported in it. Each `lez` PR body links the upstream PRs it ports, and `CHANGELOG.md` links them per release. The other 24 were opened after the sweep, or missed by it, and are triaged in §3.
- **Issues (291 open)**: the 2026-08-24 sweep read every open bug report, and the 2026-08-25 sweep every other open issue. What they found is in §5, together with the reports re-run against v0.28.5 on 2026-10-05. Their verdicts on the issues they did not name were never written into this repository, so §5 ends with a list of those issues.

### Issue Audit Order

Work an unread upstream report in this order:
1. **Reproduce against our binary before reading further.** Most reports die here — fixed through another port, and issue titles can be misleading.
2. **Read the body and entire discussion, not just the title.**
3. **Drop eza's internal infrastructure issues** (winget, `deb.gierens.de`, upstream flake, trycmd on ZFS).
4. **Group the rest by theme, one PR per theme.**

### Refreshing This File

An upstream PR or issue counts as recorded when this file, a `lez` PR body or `CHANGELOG.md` names it. On a refresh, list what is open upstream, drop what is recorded, and triage the rest into §3 and §5. Update the counts and the date at the top of this section.

---

## 2. Upstream PRs Deliberately Not Ported

Declined in the 2026-08-24 sweep. All 42 were still open on 2026-10-05, and each reason still holds. PRs opened since are triaged in §3.

| Reason | Upstream PRs |
|---|---|
| **Dependency bumps.** We maintain our own `Cargo.lock` and pins, several already ahead of upstream. | [#1543](https://github.com/eza-community/eza/pull/1543), [#1554](https://github.com/eza-community/eza/pull/1554), [#1575](https://github.com/eza-community/eza/pull/1575), [#1607](https://github.com/eza-community/eza/pull/1607), [#1659](https://github.com/eza-community/eza/pull/1659), [#1660](https://github.com/eza-community/eza/pull/1660), [#1666](https://github.com/eza-community/eza/pull/1666), [#1703](https://github.com/eza-community/eza/pull/1703), [#1745](https://github.com/eza-community/eza/pull/1745), [#1749](https://github.com/eza-community/eza/pull/1749) |
| **Infrastructure specific to eza.** Crane migration, the `deb.gierens.de` APT matrix, Miri, `cargo shear`, their RISC-V and musl release targets, their cross-build containers. | [#462](https://github.com/eza-community/eza/pull/462), [#971](https://github.com/eza-community/eza/pull/971), [#972](https://github.com/eza-community/eza/pull/972), [#1537](https://github.com/eza-community/eza/pull/1537), [#1538](https://github.com/eza-community/eza/pull/1538), [#1629](https://github.com/eza-community/eza/pull/1629), [#1753](https://github.com/eza-community/eza/pull/1753), [#1777](https://github.com/eza-community/eza/pull/1777), [#1861](https://github.com/eza-community/eza/pull/1861), [#1869](https://github.com/eza-community/eza/pull/1869), [#1890](https://github.com/eza-community/eza/pull/1890), [#1901](https://github.com/eza-community/eza/pull/1901) |
| **eza's own branding and README.** [#1713](https://github.com/eza-community/eza/pull/1713) renames leftover `exa` strings, which was done for `lez` in PR #37. | [#1625](https://github.com/eza-community/eza/pull/1625), [#1713](https://github.com/eza-community/eza/pull/1713), [#1755](https://github.com/eza-community/eza/pull/1755), [#1756](https://github.com/eza-community/eza/pull/1756), [#1914](https://github.com/eza-community/eza/pull/1914) |
| **Already covered by a port we took.** [#913](https://github.com/eza-community/eza/pull/913) ← [#925](https://github.com/eza-community/eza/pull/925) (WSL hyperlinks), [#1596](https://github.com/eza-community/eza/pull/1596) ← [#1923](https://github.com/eza-community/eza/pull/1923) (stdin), [#1838](https://github.com/eza-community/eza/pull/1838)/[#1840](https://github.com/eza-community/eza/pull/1840)/[#1844](https://github.com/eza-community/eza/pull/1844) ← [#1848](https://github.com/eza-community/eza/pull/1848) (all three are the same non-UTF-8 `--time-style` fix), [#1233](https://github.com/eza-community/eza/pull/1233)/[#1504](https://github.com/eza-community/eza/pull/1504) ← commit `cfe0abb7`, which removed the Windows `_`-prefix filter outright. | [#913](https://github.com/eza-community/eza/pull/913), [#1233](https://github.com/eza-community/eza/pull/1233), [#1504](https://github.com/eza-community/eza/pull/1504), [#1596](https://github.com/eza-community/eza/pull/1596), [#1838](https://github.com/eza-community/eza/pull/1838), [#1840](https://github.com/eza-community/eza/pull/1840), [#1844](https://github.com/eza-community/eza/pull/1844) |
| **Dead or disproportionate.** [#974](https://github.com/eza-community/eza/pull/974) is ±25k lines of churn under `CHANGES_REQUESTED`; [#1765](https://github.com/eza-community/eza/pull/1765) touches 744 files; [#936](https://github.com/eza-community/eza/pull/936) fixes clippy warnings we do not have; [#575](https://github.com/eza-community/eza/pull/575) waits on an upstream design decision that never came; [#1658](https://github.com/eza-community/eza/pull/1658) conflicts. | [#575](https://github.com/eza-community/eza/pull/575), [#936](https://github.com/eza-community/eza/pull/936), [#974](https://github.com/eza-community/eza/pull/974), [#1658](https://github.com/eza-community/eza/pull/1658), [#1765](https://github.com/eza-community/eza/pull/1765) |
| **Product decisions, not oversights.** [#770](https://github.com/eza-community/eza/pull/770) (`--no-header`) superseded by configuration file support (`[display] header = false`); [#1903](https://github.com/eza-community/eza/pull/1903) was solved our own way in PR #26. | [#770](https://github.com/eza-community/eza/pull/770), [#1804](https://github.com/eza-community/eza/pull/1804), [#1903](https://github.com/eza-community/eza/pull/1903) |

---

## 3. Upstream PRs Opened Since the Sweep

Triaged 2026-10-05. [#1729](https://github.com/eza-community/eza/pull/1729) is older but was missed by the 2026-08-24 sweep.

| Upstream PR | Verdict |
|---|---|
| [#1729](https://github.com/eza-community/eza/pull/1729) man page cross-references ([#981](https://github.com/eza-community/eza/issues/981)) | **Covered.** PR #29 made `SEE ALSO` use man notation; see #981 in §5. |
| [#1926](https://github.com/eza-community/eza/pull/1926) Lua block comments ([#1918](https://github.com/eza-community/eza/issues/1918)) | **Covered.** `--code` counts a two-line `--[[ … ]]` comment, a `--` comment and one statement as 3 comment lines and 1 code line. |
| [#1928](https://github.com/eza-community/eza/pull/1928) README demo link | **Declined.** eza's own README. |
| [#1930](https://github.com/eza-community/eza/pull/1930) compose file type filters ([#1927](https://github.com/eza-community/eza/issues/1927)) | **Ported** in PR #93. |
| [#1931](https://github.com/eza-community/eza/pull/1931) repeatable zsh completions ([#1846](https://github.com/eza-community/eza/issues/1846)) | **Ported** in PR #93. |
| [#1932](https://github.com/eza-community/eza/pull/1932) star-history chart | **Declined.** eza's own CI. |
| [#1934](https://github.com/eza-community/eza/pull/1934) naersk bump for crates.io 403s | **Declined.** eza's own Nix packaging. |
| [#1935](https://github.com/eza-community/eza/pull/1935) `--total-size` over visible files only | **Covered.** Commit `b4e0a3d4` (PR #38); see #1498 in §5. |
| [#1938](https://github.com/eza-community/eza/pull/1938) `use Self` | **Declined.** Churn with no change in behaviour. |
| [#1939](https://github.com/eza-community/eza/pull/1939), [#1944](https://github.com/eza-community/eza/pull/1944) split `filter_child_files` ([#1927](https://github.com/eza-community/eza/issues/1927)) | **Covered** by the #1930 port (PR #93). |
| [#1942](https://github.com/eza-community/eza/pull/1942) reject `--width` above 65535 ([#1895](https://github.com/eza-community/eza/issues/1895)) | **Covered.** The width clamp ported from [#1909](https://github.com/eza-community/eza/pull/1909) in PR #3; `-w 99999999999` lists normally. |
| [#1943](https://github.com/eza-community/eza/pull/1943) directory-only names to `DIRECTORY_ICONS` ([#1940](https://github.com/eza-community/eza/issues/1940)) | **Ported** in commit `56cef37e`. |
| [#1945](https://github.com/eza-community/eza/pull/1945) named themes (`EZA_THEME`) | **Covered**, more widely: `--theme`, `LEZ_THEME` or `EZA_THEME`, and `[theme] name`; see #1945 in §5. |
| [#1946](https://github.com/eza-community/eza/pull/1946) one spelling of "color" in help ([#1243](https://github.com/eza-community/eza/issues/1243)) | **Covered**; see #1243 in §5. |
| [#1947](https://github.com/eza-community/eza/pull/1947) `--only-files`/`--only-dirs` with `-d` ([#618](https://github.com/eza-community/eza/issues/618)) | **Covered.** `lez -d -f dir file link` prints only `file`. |
| [#1948](https://github.com/eza-community/eza/pull/1948) `--user-length` | **Covered** by `--owner-width`, which cuts group names too; see #1760 in §5. |
| [#1949](https://github.com/eza-community/eza/pull/1949) `--finder-meta` | **Partly covered.** `-e`/`--tags` lists Finder colour tags; the SF Symbol overlays are declined (§5). |
| [#1950](https://github.com/eza-community/eza/pull/1950) trailing separators on Windows ([#404](https://github.com/eza-community/eza/issues/404)) | **Covered.** #404 was verified fixed on a `windows-latest` runner in issue #57. |
| [#1951](https://github.com/eza-community/eza/pull/1951) no panic on a closed stdout pipe on Windows | **Not yet checked.** Needs the Windows probe (§4). |
| [#1952](https://github.com/eza-community/eza/pull/1952) bounded `listxattr` retries on macOS ([#1850](https://github.com/eza-community/eza/issues/1850)) | **Covered.** Commit `7a3487d7` (PR #38) caps the `ERANGE` retry loop. The macFUSE hang in #1850 itself is out of reach (§4). |
| [#1953](https://github.com/eza-community/eza/pull/1953) wildcard expansion on Windows ([#337](https://github.com/eza-community/eza/issues/337)) | **Covered.** PR #61. |
| [#1955](https://github.com/eza-community/eza/pull/1955) `--total-size -aa` skips `..` ([#1690](https://github.com/eza-community/eza/issues/1690)) | **Covered.** `lez -aal --total-size` prints `-` as the size of `..`. |
| [#1956](https://github.com/eza-community/eza/pull/1956) `.tzst` extension | **Ported**, with tests, by `feat(icons): treat .tzst as a compressed tarball`. |

---

## 4. Platform Capabilities & Testing Scope

- **Linux containers on macOS**: A Linux-only report is reproducible locally in minutes — build inside `rust:*-bookworm` with `CARGO_TARGET_DIR=/tmp/target` and a mounted read-only checkout.
- **Windows testing**: Windows cannot run locally in containers; use the `Windows Repro Probe` manual GitHub workflow (`.github/workflows/windows-probe.yml`).
- **Nix flake check on macOS**: `nix flake check` does not pass on macOS due to OS differences (`nixbld` vs `_nixbld1` user, macOS `@` xattr marker). Use CI for Nix flake check validation; `nix develop` and `nix build` work fine locally.
- **Out of reach**: Specific physical USB hardware, CIFS/macFUSE mounts, systemd-homed, iSH, and terminal font glyphs.

---

## 5. Upstream Issues & Status

### Still Open Upstream (Known Status)

| Upstream ID | State & Notes |
|---|---|
| [#875](https://github.com/eza-community/eza/issues/875) | Alpine on iSH. Needs iSH; no code lead. |
| [#1428](https://github.com/eza-community/eza/issues/1428) | systemd-homed user names. We already resolve through `uzers` → `getpwuid_r` → NSS, which is the right API; nothing to act on without the setup. |
| [#1500](https://github.com/eza-community/eza/issues/1500) | Hang on one USB device. Needs that device. |
| [#743](https://github.com/eza-community/eza/issues/743) | `--color-scale` under tmux on Wayland. The flat gradient was ours and is fixed; whether truecolor also fails there is unproven. |
| [#844](https://github.com/eza-community/eza/issues/844), [#1214](https://github.com/eza-community/eza/issues/1214), [#1378](https://github.com/eza-community/eza/issues/1378), [#1710](https://github.com/eza-community/eza/issues/1710) | Performance at 200k+ files. Does not reproduce at 12k; see issue #59. |
| [#337](https://github.com/eza-community/eza/issues/337), [#404](https://github.com/eza-community/eza/issues/404), [#853](https://github.com/eza-community/eza/issues/853), [#1025](https://github.com/eza-community/eza/issues/1025), [#1104](https://github.com/eza-community/eza/issues/1104), [#1220](https://github.com/eza-community/eza/issues/1220), [#1665](https://github.com/eza-community/eza/issues/1665), [#1769](https://github.com/eza-community/eza/issues/1769) | Windows. Settled — see issue #57. |

### Feature Requests Already Delivered in `lez`

| Upstream ID | What answers it in `lez` |
|---|---|
| [#341](https://github.com/eza-community/eza/issues/341), [#1847](https://github.com/eza-community/eza/issues/1847) | `--summary`, `--print-total` |
| [#420](https://github.com/eza-community/eza/issues/420) | `--ignore-submodule-contents` |
| [#443](https://github.com/eza-community/eza/issues/443), [#710](https://github.com/eza-community/eza/issues/710) | `--no-extended` |
| [#472](https://github.com/eza-community/eza/issues/472), [#768](https://github.com/eza-community/eza/issues/768) | `--json` — keyed objects, one per entry |
| [#516](https://github.com/eza-community/eza/issues/516) | `--show-symlinks` / `--no-symlinks` |
| [#520](https://github.com/eza-community/eza/issues/520), [#1003](https://github.com/eza-community/eza/issues/1003) | `--spacing` |
| [#589](https://github.com/eza-community/eza/issues/589) | `--warn-hidden` |
| [#630](https://github.com/eza-community/eza/issues/630) | `--help` shows `--color-scale[=<FIELDS>...]` with its values |
| [#653](https://github.com/eza-community/eza/issues/653) | timezone offsets track DST; verified Feb/Jun/Nov across `TZ` |
| [#736](https://github.com/eza-community/eza/issues/736), [#980](https://github.com/eza-community/eza/issues/980), [#1573](https://github.com/eza-community/eza/issues/1573) | `-t` / `-lt` / `-lrt` match `/bin/ls` byte for byte (`normalize_short_time_arg`) |
| [#889](https://github.com/eza-community/eza/issues/889) | `--octal-permissions --no-permissions` |
| [#921](https://github.com/eza-community/eza/issues/921) | `-d` with `--stdin` |
| [#948](https://github.com/eza-community/eza/issues/948) | `--cachedir-ignore` |
| [#981](https://github.com/eza-community/eza/issues/981) | `--absolute` documented, `SEE ALSO` uses man notation, `$version` is substituted by `just man` |
| [#1042](https://github.com/eza-community/eza/issues/1042), [#1683](https://github.com/eza-community/eza/issues/1683), [#1737](https://github.com/eza-community/eza/issues/1737) | Jenkinsfile, `Icon\r`, bicep and bicepparam icons |
| [#1073](https://github.com/eza-community/eza/issues/1073) | `--mime-types` |
| [#1090](https://github.com/eza-community/eza/issues/1090) | `--absolute` |
| [#1123](https://github.com/eza-community/eza/issues/1123) | `--quotes=always` |
| [#1141](https://github.com/eza-community/eza/issues/1141) | multiple path arguments obey `--sort` |
| [#1219](https://github.com/eza-community/eza/issues/1219) | `-@` decodes `security.capability` into `cap_…=eip` through `capctl`, and PR #66 added the `ca` styling |
| [#1446](https://github.com/eza-community/eza/issues/1446), [#1778](https://github.com/eza-community/eza/issues/1778) | `--ignore-glob '**/dir/*'` hides contents and keeps the directory |
| [#1484](https://github.com/eza-community/eza/issues/1484) | `completions/pwsh` |
| [#1540](https://github.com/eza-community/eza/issues/1540) | `--no-symlink-targets` |
| [#1616](https://github.com/eza-community/eza/issues/1616) | `-H` |
| [#1657](https://github.com/eza-community/eza/issues/1657) | `--time-style relative-recent` |
| [#1734](https://github.com/eza-community/eza/pull/1734) | `libgit2-sys 0.18.5+1.9.4`, past the 1.9.2 advisories |
| [#1746](https://github.com/eza-community/eza/issues/1746) | `--follow-symlinks`, `-X` |
| [#1750](https://github.com/eza-community/eza/issues/1750) | `--ignore-glob-ci` |
| [#1773](https://github.com/eza-community/eza/pull/1773) | `--hyperlink[=WHEN]` |
| [#1835](https://github.com/eza-community/eza/issues/1835) | `--sort=path` |
| [#1904](https://github.com/eza-community/eza/issues/1904) | `Dir::contains` memoises into a set; 5000 `.log` files list in 0.07s |
| [#1912](https://github.com/eza-community/eza/pull/1912) | `palette_derive` pinned to `=0.7.5` beside `palette` |
| [#223](https://github.com/eza-community/eza/issues/223), [#579](https://github.com/eza-community/eza/issues/579) | the Git column is dropped when nothing in the listing is in a repo |
| [#1735](https://github.com/eza-community/eza/issues/1735) | First-class `FileType::Data`, `dt` in `LEZ_COLORS`, `file_type.data` theme style, with `.parquet`, `.h5`, `.npy`, `.csv`, `.tsv`, `.sqlite`, and scientific data formats |
| [#1768](https://github.com/eza-community/eza/issues/1768), [#1417](https://github.com/eza-community/eza/issues/1417), [#1720](https://github.com/eza-community/eza/issues/1720) | Granular Windows hidden-entry visibility controls: `--show-dotfiles`, `--no-system` (alias `--hide-system`), `--no-hidden-attrib` (alias `--hide-hidden-attrib`), `--no-hidden-links` (alias `--no-junctions`), and `config.toml` filter options. |
| [#139](https://github.com/eza-community/eza/issues/139), [#766](https://github.com/eza-community/eza/issues/766), [#770](https://github.com/eza-community/eza/pull/770), [#812](https://github.com/eza-community/eza/issues/812), [#1587](https://github.com/eza-community/eza/issues/1587), [#1707](https://github.com/eza-community/eza/issues/1707), [#1875](https://github.com/eza-community/eza/issues/1875) | Configuration file for option defaults (`config.toml`, `.lez.toml`), `--config`, `--no-config`, `LEZ_CONFIG_FILE` |
| [#1823](https://github.com/eza-community/eza/issues/1823) | Configurable Git indicator status glyphs and repository status glyphs in `theme.yml` under `git` and `git_repo` (`glyph: "..."`) alongside `--git-glyphs`. |
| [#1466](https://github.com/eza-community/eza/issues/1466) | `--context` / `-Z` SELinux MCS translation support (`selinux_raw_to_trans_context`) on Linux. |
| [#1933](https://github.com/eza-community/eza/issues/1933) | `--no-language` flag and `[loc] language = false` configuration to suppress Language column when `--loc` is enabled. |
| [#1927](https://github.com/eza-community/eza/issues/1927), [#1930](https://github.com/eza-community/eza/pull/1930) | Independent composable file type filters (`--only-files`, `--only-dirs`, `--no-symlinks`, `--show-symlinks`), resolving filter cancellation when combining flags (PR #93). |
| [#1846](https://github.com/eza-community/eza/issues/1846), [#1931](https://github.com/eza-community/eza/pull/1931) | Repeatable options in Zsh completions (`completions/zsh/_lez` and `_eza`) via `*` prefix for all options (PR #93). |
| [#1498](https://github.com/eza-community/eza/issues/1498), [#1291](https://github.com/eza-community/eza/issues/1291) | Recursive size calculation (`--total-size`) respects `--all` / `dot_filter`, skipping hidden directories when dotfiles are not shown (commit `b4e0a3d4`, PR #38). The 2026-08-25 sweep recorded #1498 as still reproducing; that was wrong. A build of the PR #38 merge already gives `1.0k` without `-a` and `101k` with it, for a directory holding a 100 kB hidden file. |
| [#1922](https://github.com/eza-community/eza/issues/1922) | `--code` counts the lines in a Markdown fence under the fence's language (PRs #109, #111) |
| [#1929](https://github.com/eza-community/eza/issues/1929) | the release workflow builds `aarch64-pc-windows-msvc` |
| [#1243](https://github.com/eza-community/eza/issues/1243), [#1946](https://github.com/eza-community/eza/pull/1946) | `--help`, the shell completions and the option lists in the README and `lez(1)` spell it "color", and `--help` shows the `--colour`, `--colour-scale` and `--colour-scale-mode` aliases, so searching it for either spelling finds every colour option. Prose outside those option lists, such as `lez_colors(5)`, keeps its own spelling. |
| [#584](https://github.com/eza-community/eza/issues/584) | `-N`/`--literal`, the same as `--quotes=never`, and `QUOTING_STYLE`, read after `LEZ_QUOTING_STYLE` in GNU's words: `literal` is `never`, `shell` and `shell-escape` are `auto`, `shell-always` and `shell-escape-always` are `always`. GNU's `c`, `escape` and `locale` have no counterpart and are passed over. |
| [#1760](https://github.com/eza-community/eza/issues/1760), [#1948](https://github.com/eza-community/eza/pull/1948) | `--owner-width=COLS` (or `owner_width`) cuts user and group names wider than `COLS` columns, ending them in `…`; numbers and `--json` stay whole, and `--smart-group` compares the whole names. Cutting at a character (`@`), which #1760 also floated, is not offered. |
| [#1920](https://github.com/eza-community/eza/issues/1920) | `--group-dotfiles-first` lists dotfiles before the rest within each directory group, so `--group-directories-first --group-dotfiles-first --sort=extension` lists directories, then dotfiles, then the other files by extension. |
| [#1945](https://github.com/eza-community/eza/pull/1945) | `--theme=NAME`, `LEZ_THEME` (or `EZA_THEME`) and `name` under `[theme]` read `themes/NAME.yml` or `.yaml` from the configuration directory instead of `theme.yml`; a name holding a directory is a path, and a theme that is not there is an error. |
| [#1954](https://github.com/eza-community/eza/issues/1954) | `--explain` lists each entry with the rule that chose its name's colour (the glob and the variable it was in, the theme entry, the built-in file type, or a file kind such as `di` or `ex`) and the one that chose its icon. It covers the name and the icon, not every column. |
| [#600](https://github.com/eza-community/eza/issues/600) | `--inspect-archives` lists `.tar` and `.zip` entries; a `.zip` is read from its central directory, ZIP64 included, with nothing decompressed. Compressed tarballs (`.tar.gz`, `.tar.xz`, `.tar.bz2`), also named in the issue, are not opened. |

### Reproduced and Fixed in `lez`

| Upstream ID | What was fixed |
|---|---|
| [#1571](https://github.com/eza-community/eza/issues/1571) | `--flags` / `-O` extended to Linux to read inode flags/attributes (`FS_IOC_GETFLAGS` / `lsattr`) in long and short format. |
| [#509](https://github.com/eza-community/eza/issues/509), [#1743](https://github.com/eza-community/eza/issues/1743), [#1448](https://github.com/eza-community/eza/issues/1448), [#1892](https://github.com/eza-community/eza/issues/1892) | `natord` was the only comparator, breaking `LC_ALL=C` hex sort. `--sort=lexicographic` added for plain comparison. |
| [#1868](https://github.com/eza-community/eza/issues/1868) | `--code` skipped dotfiles and `-a` did nothing. Fixed to count correctly with `--loc`. |
| [#922](https://github.com/eza-community/eza/issues/922), [#558](https://github.com/eza-community/eza/issues/558) | Syscall overhead (buffered stdout write and unnecessary stat when colors were off). Reduced 5000 stats to 1 on `lez -1 dir \| wc -l`. With colours on the stat stays, as it does for `ls`. #558 also counted a `readlinkat` and a `getcwd` per entry; those were measured on Linux by the reporter and have not been re-measured here, so treat them as unaccounted for rather than fixed. |
| [#1642](https://github.com/eza-community/eza/issues/1642), [#1640](https://github.com/eza-community/eza/issues/1640) | `--blocks` CLI flag added for integer filesystem block count column alongside `-S` / `--blocksize` (byte size), and `bl` in `LS_COLORS` now styles it instead of the file size palette. #1640 itself was closed upstream on 2025-10-15. |
| [#765](https://github.com/eza-community/eza/issues/765) | `mh` accepted and properly parsed. |
| [#1002](https://github.com/eza-community/eza/issues/1002), [#745](https://github.com/eza-community/eza/issues/745) | High stat overhead on empty directory glyph probing over FUSE/NFS. `LEZ_NO_EMPTY_DIR_ICON` avoids probing. |
| [#1732](https://github.com/eza-community/eza/issues/1732) | `--size-digits=<NUM>` (alias `--digits`) and `LEZ_SIZE_DIGITS` added to customize size column precision/digit count. |
| [#728](https://github.com/eza-community/eza/issues/728), [#730](https://github.com/eza-community/eza/pull/730), [#1791](https://github.com/eza-community/eza/pull/1791) | Cohesive full-path quoting (`'/path/with spaces/file.txt'`) and configurable quote styling via `qu` code in `LEZ_COLORS` & `theme.yml`. |
| [#1940](https://github.com/eza-community/eza/issues/1940), [#1943](https://github.com/eza-community/eza/pull/1943) | Four `FILENAME_ICONS` entries (`.atom`, `.idea`, `.rvm`, `.zsh_sessions`) unreachable because names are directories. Moved to `DIRECTORY_ICONS`. |
| [#1088](https://github.com/eza-community/eza/issues/1088) | `ca` in `LS_COLORS` was unsupported; PR #66 styles files with capabilities by it (issue #60). Verified on Linux: after `setcap cap_net_raw+ep`, `LS_COLORS='ca=30;41'` paints the name `41;30`. |
| [#1548](https://github.com/eza-community/eza/issues/1548) | Names with control characters were printed as `'with\u{1b}control 1'` or a bare `with\nnewline`, which a shell cannot read back. Since v0.28.5 (PR #149), `--quotes=auto` writes them in ANSI-C quotes (`$'with\nnewline'`, `$'with\033control 1'`). GNU `ls` quotes only the special characters (`'with'$'\n''newline'`); both read back as the same name. Distinct from [#1482](https://github.com/eza-community/eza/issues/1482), fixed in PR #52. |
| [#1576](https://github.com/eza-community/eza/issues/1576) | Icons ignored case but `LS_COLORS` globs did not. Globs now ignore case too (commit `ac07744f`), and, as in GNU `ls` since coreutils 9.2, two globs that differ only in case and give different colours each match only their own case: `*.qux=31:*.QUX=32` paints `a.qux` red and `b.QUX` green, and leaves `c.Qux` uncoloured, as GNU `ls` 9.4 does. Globs in `LEZ_COLORS` are weighed with those in `LS_COLORS` as one list. |
| [#1700](https://github.com/eza-community/eza/issues/1700), [#1224](https://github.com/eza-community/eza/issues/1224) | A `theme.yml` lost to `LS_COLORS`, which the system's `dircolors` often sets for every program: under its `di=01;34`, a theme's red directories turned blue. The rule was mixed, too: a theme's `extensions` entry beat a `LEZ_COLORS` glob, while its `filekinds` lost to both variables. A theme file now takes the place of `LS_COLORS`, which is not read beside one, and `LEZ_COLORS` or `EZA_COLORS` is laid over the theme, globs included. The rest of #1224 does not reproduce on Linux: `XDG_CONFIG_HOME`, a `~` in `EZA_CONFIG_DIR` and a symlinked `theme.yml`, as home-manager writes, are all read. On macOS without `XDG_CONFIG_HOME`, `lez` looks in `~/Library/Application Support/lez`, as `lez_colors-explanation(5)` says; that was not checked on macOS here. |

### Re-run 2026-10-05: Open Upstream Bugs That Do Not Reproduce

Run against v0.28.5 on Linux. None of these was named in this file before; most were fixed by ports listed in `CHANGELOG.md`.

| Upstream ID | Checked with | Result |
|---|---|---|
| [#521](https://github.com/eza-community/eza/issues/521) | `.gitignore` holding a lone `*`, `-l --git` | every file marked `I` (PR #47) |
| [#618](https://github.com/eza-community/eza/issues/618) | `-f dir1 dir2`, `-d -f dir file link` | only regular files listed |
| [#662](https://github.com/eza-community/eza/issues/662), [#1021](https://github.com/eza-community/eza/issues/1021) | `-1 link` into a pipe | the link name alone, no `-> target` |
| [#832](https://github.com/eza-community/eza/issues/832) | `-FT dir` | the full tree |
| [#923](https://github.com/eza-community/eza/issues/923) | `-ld --total-size` over a 50 kB file with two hard links | `50k`, counted once |
| [#995](https://github.com/eza-community/eza/issues/995) | `-T --absolute /full/path` | an absolute path on every row |
| [#1085](https://github.com/eza-community/eza/issues/1085) | `-la --git-repos` at a repository root | `.git` gets no repository status |
| [#1086](https://github.com/eza-community/eza/issues/1086) | `--git-ignore` on a directory with its own `.gitignore` | the ignored file is hidden |
| [#1116](https://github.com/eza-community/eza/issues/1116) | a directory named after a theme `filenames` entry | keeps the folder icon |
| [#1240](https://github.com/eza-community/eza/issues/1240) | `--time-style=+%Q` | usage error, exit 3, no panic |
| [#1339](https://github.com/eza-community/eza/issues/1339) | `-l --inode` | same inode as `ls -i` |
| [#1499](https://github.com/eza-community/eza/issues/1499) | `-T -L 1` | one level |
| [#1568](https://github.com/eza-community/eza/issues/1568), [#1725](https://github.com/eza-community/eza/issues/1725) | no arguments into a pipe; stdin from `/dev/null` | the working directory is listed |
| [#1590](https://github.com/eza-community/eza/issues/1590), [#1941](https://github.com/eza-community/eza/issues/1941) | theme `filenames` icon with `--color=never`; theme found through `LEZ_CONFIG_DIR` and `EZA_CONFIG_DIR` | icons and colours applied |
| [#1633](https://github.com/eza-community/eza/issues/1633), [#1728](https://github.com/eza-community/eza/issues/1728) | `--hyperlink=always` on `dir/a#b?c[1].txt` | full path, `#`, `?`, `[`, `]` percent-encoded |
| [#1646](https://github.com/eza-community/eza/issues/1646) | `--grid --long --across` | filled across |
| [#1654](https://github.com/eza-community/eza/issues/1654) | `👨‍👩‍👧` as a theme glyph | printed whole |
| [#1668](https://github.com/eza-community/eza/issues/1668) | a file dated 1960 | `1 Jan 1960` |
| [#1673](https://github.com/eza-community/eza/issues/1673) | `--git-ignore` inside a bare repository | lists normally |
| [#1682](https://github.com/eza-community/eza/issues/1682) | `LS_COLORS='su=31'` and `'tw=31'` beside files named `su` and `tw` | the files stay uncoloured |
| [#1690](https://github.com/eza-community/eza/issues/1690) | `-aal --total-size` | `-` as the size of `..` |
| [#1701](https://github.com/eza-community/eza/issues/1701) | `-R -L 1 dir` | one level |
| [#1740](https://github.com/eza-community/eza/issues/1740) | `-t dir` | `dir` is read as a path |
| [#1772](https://github.com/eza-community/eza/issues/1772) | `--sort=newest` | newest first |
| [#1789](https://github.com/eza-community/eza/issues/1789) | `LEZ_CONFIG_DIR='~/cfg'` | `~` expanded, theme applied |
| [#1837](https://github.com/eza-community/eza/issues/1837) | non-UTF-8 `--time-style` | usage error, exit 3, no panic |
| [#1842](https://github.com/eza-community/eza/issues/1842) | `--icons=auto` into a pipe with `COLUMNS=80` | no icons |
| [#1863](https://github.com/eza-community/eza/issues/1863) | `--color=automatic` | accepted |
| [#1864](https://github.com/eza-community/eza/issues/1864), [#1921](https://github.com/eza-community/eza/issues/1921) | `--icons dir`, `--icons dir/` | `dir` is read as a path |
| [#1874](https://github.com/eza-community/eza/issues/1874) | `LEZ_STRICT=true` / `EZA_STRICT=true` with the reporter's flags | no "binary is useless" error |
| [#1879](https://github.com/eza-community/eza/issues/1879) | `--hyperlink dir` | `dir` is read as a path |
| [#1885](https://github.com/eza-community/eza/issues/1885) | `-alF .` | lists normally |
| [#1895](https://github.com/eza-community/eza/issues/1895) | `-w 99999999999` | no panic |
| [#1900](https://github.com/eza-community/eza/issues/1900) | `--binary --bytes` | the last one wins |
| [#1918](https://github.com/eza-community/eza/issues/1918) | Lua `--[[ … ]]` under `--code` | counted as comment |

### Reproduced but Still Open / By Design

| Upstream ID | Description & Status |
|---|---|
| [#1919](https://github.com/eza-community/eza/issues/1919) | `.m` renders as C icon (shared with Objective-C / MATLAB; no MATLAB glyph in Nerd Fonts). |

### Pending Decisions

Real requests that `lez` has not answered either way, each needing a product decision before any code. None is open: the last seven were settled on 2026-10-05, and their outcomes are in the tables above and below.

### Specifically Audited & Declined Issues (Detailed Rationale)

| Upstream ID | Description & Proof for Declining |
|---|---|
| [#693](https://github.com/eza-community/eza/issues/693) | **`--hyperlink` "eating characters" when saved to shell variable.** Not a bug in `lez`. It ends OSC 8 with ST (`ESC \`: `^[]8;;file://…/1^[\1^[]8;;^[\`), as GNU `ls` has since coreutils 9.10 (February 2026); 9.9 and earlier end it with BEL. The character eating occurs when `echo` interprets `\a`, `\b`, `\t` from combining the terminator with the leading letter of the filename. Changing to BEL would deviate from GNU ls standard. |
| [#1360](https://github.com/eza-community/eza/issues/1360) | **Subdirectory `.gitignore` behavior.** In `lez`, explicit target arguments always display their contents (`tests/git/gitignore.rs` locks in "explicit arguments override filters"). Proposed `--no-git-ignore` was also declined upstream. |
| [#519](https://github.com/eza-community/eza/issues/519) | **96G vs 103G.** By design, not an oversight. `lez` defaults to SI units ($1000^3$), while `--binary` gives 96Gi ($1024^3$). Matches upstream standard and documentation. |
| [#1949](https://github.com/eza-community/eza/pull/1949) | **SF Symbol overlays for Finder metadata on macOS (`--finder-meta`).** The PR draws a folder's SF Symbol as one of about 230 emoji picked by hand to look like it, a table that would have to follow each macOS release and that cannot be checked from here. Finder's colour tags, the other half of the PR, are already listed by `-e`/`--tags`. |

### General Declined / Not Applicable Issues

- **Upstream Packaging & Infrastructure**: [#347](https://github.com/eza-community/eza/issues/347), [#475](https://github.com/eza-community/eza/issues/475), [#601](https://github.com/eza-community/eza/issues/601), [#613](https://github.com/eza-community/eza/issues/613) (trycmd on ZFS), [#646](https://github.com/eza-community/eza/issues/646), [#919](https://github.com/eza-community/eza/issues/919), [#930](https://github.com/eza-community/eza/issues/930), [#951](https://github.com/eza-community/eza/issues/951), [#973](https://github.com/eza-community/eza/issues/973) (their `idump`), [#985](https://github.com/eza-community/eza/issues/985) (`deb.gierens.de` build), [#1033](https://github.com/eza-community/eza/issues/1033), [#1040](https://github.com/eza-community/eza/issues/1040), [#1185](https://github.com/eza-community/eza/issues/1185) (winget), [#1372](https://github.com/eza-community/eza/issues/1372), [#1561](https://github.com/eza-community/eza/issues/1561), [#1605](https://github.com/eza-community/eza/issues/1605), [#1610](https://github.com/eza-community/eza/issues/1610), [#1619](https://github.com/eza-community/eza/issues/1619), [#1622](https://github.com/eza-community/eza/issues/1622), [#1871](https://github.com/eza-community/eza/issues/1871) (their flake on Darwin), [#1621](https://github.com/eza-community/eza/issues/1621), [#1670](https://github.com/eza-community/eza/issues/1670), [#1748](https://github.com/eza-community/eza/issues/1748), [#1776](https://github.com/eza-community/eza/issues/1776), [#1876](https://github.com/eza-community/eza/issues/1876), [#1888](https://github.com/eza-community/eza/issues/1888) (their `--version` snapshot), doc issues [#969](https://github.com/eza-community/eza/issues/969), [#1099](https://github.com/eza-community/eza/issues/1099), [#1100](https://github.com/eza-community/eza/issues/1100), [#1487](https://github.com/eza-community/eza/issues/1487), governance [#1872](https://github.com/eza-community/eza/issues/1872).
- **Declined based on discussion**: [#479](https://github.com/eza-community/eza/issues/479) (PGO speedup minimal), [#756](https://github.com/eza-community/eza/issues/756) (reading file contents declined), [#729](https://github.com/eza-community/eza/issues/729) (`lib.rs` is internal), [#1117](https://github.com/eza-community/eza/issues/1117) (Unicode glyph rendering issues), [#905](https://github.com/eza-community/eza/issues/905)/[#1401](https://github.com/eza-community/eza/issues/1401) (blocked on Windows ACL and libgit2 sha256).

### Not Recorded Here

On 2026-10-05, 93 open upstream issues were named neither here nor in a `lez` PR body or `CHANGELOG.md`.

- **23 bug reports.** The 2026-08-24 sweep read them, but their verdicts were not written down, and they were not re-run on 2026-10-05. Reproduce each before acting on it (§1): [#541](https://github.com/eza-community/eza/issues/541), [#576](https://github.com/eza-community/eza/issues/576), [#585](https://github.com/eza-community/eza/issues/585), [#682](https://github.com/eza-community/eza/issues/682), [#755](https://github.com/eza-community/eza/issues/755), [#827](https://github.com/eza-community/eza/issues/827), [#857](https://github.com/eza-community/eza/issues/857), [#865](https://github.com/eza-community/eza/issues/865), [#874](https://github.com/eza-community/eza/issues/874), [#1054](https://github.com/eza-community/eza/issues/1054), [#1361](https://github.com/eza-community/eza/issues/1361), [#1444](https://github.com/eza-community/eza/issues/1444), [#1489](https://github.com/eza-community/eza/issues/1489), [#1569](https://github.com/eza-community/eza/issues/1569), [#1578](https://github.com/eza-community/eza/issues/1578), [#1592](https://github.com/eza-community/eza/issues/1592), [#1623](https://github.com/eza-community/eza/issues/1623), [#1648](https://github.com/eza-community/eza/issues/1648), [#1674](https://github.com/eza-community/eza/issues/1674), [#1706](https://github.com/eza-community/eza/issues/1706), [#1781](https://github.com/eza-community/eza/issues/1781), [#1788](https://github.com/eza-community/eza/issues/1788), [#1873](https://github.com/eza-community/eza/issues/1873).
- **70 feature requests.** The 2026-08-25 sweep read every one and concluded that a request it did not name is a real gap and a product decision, not an oversight. Its notes on each were not kept, so treat these as undecided rather than declined: [#148](https://github.com/eza-community/eza/issues/148), [#242](https://github.com/eza-community/eza/issues/242), [#292](https://github.com/eza-community/eza/issues/292), [#309](https://github.com/eza-community/eza/issues/309), [#315](https://github.com/eza-community/eza/issues/315), [#372](https://github.com/eza-community/eza/issues/372), [#389](https://github.com/eza-community/eza/issues/389), [#559](https://github.com/eza-community/eza/issues/559), [#568](https://github.com/eza-community/eza/issues/568), [#574](https://github.com/eza-community/eza/issues/574), [#593](https://github.com/eza-community/eza/issues/593), [#647](https://github.com/eza-community/eza/issues/647), [#650](https://github.com/eza-community/eza/issues/650), [#657](https://github.com/eza-community/eza/issues/657), [#660](https://github.com/eza-community/eza/issues/660), [#683](https://github.com/eza-community/eza/issues/683), [#700](https://github.com/eza-community/eza/issues/700), [#731](https://github.com/eza-community/eza/issues/731), [#738](https://github.com/eza-community/eza/issues/738), [#751](https://github.com/eza-community/eza/issues/751), [#792](https://github.com/eza-community/eza/issues/792), [#807](https://github.com/eza-community/eza/issues/807), [#851](https://github.com/eza-community/eza/issues/851), [#854](https://github.com/eza-community/eza/issues/854), [#876](https://github.com/eza-community/eza/issues/876), [#939](https://github.com/eza-community/eza/issues/939), [#943](https://github.com/eza-community/eza/issues/943), [#976](https://github.com/eza-community/eza/issues/976), [#991](https://github.com/eza-community/eza/issues/991), [#997](https://github.com/eza-community/eza/issues/997), [#1019](https://github.com/eza-community/eza/issues/1019), [#1089](https://github.com/eza-community/eza/issues/1089), [#1094](https://github.com/eza-community/eza/issues/1094), [#1103](https://github.com/eza-community/eza/issues/1103), [#1134](https://github.com/eza-community/eza/issues/1134), [#1174](https://github.com/eza-community/eza/issues/1174), [#1213](https://github.com/eza-community/eza/issues/1213), [#1223](https://github.com/eza-community/eza/issues/1223), [#1242](https://github.com/eza-community/eza/issues/1242), [#1250](https://github.com/eza-community/eza/issues/1250), [#1329](https://github.com/eza-community/eza/issues/1329), [#1350](https://github.com/eza-community/eza/issues/1350), [#1354](https://github.com/eza-community/eza/issues/1354), [#1369](https://github.com/eza-community/eza/issues/1369), [#1385](https://github.com/eza-community/eza/issues/1385), [#1416](https://github.com/eza-community/eza/issues/1416), [#1433](https://github.com/eza-community/eza/issues/1433), [#1441](https://github.com/eza-community/eza/issues/1441), [#1471](https://github.com/eza-community/eza/issues/1471), [#1494](https://github.com/eza-community/eza/issues/1494), [#1523](https://github.com/eza-community/eza/issues/1523), [#1547](https://github.com/eza-community/eza/issues/1547), [#1584](https://github.com/eza-community/eza/issues/1584), [#1611](https://github.com/eza-community/eza/issues/1611), [#1636](https://github.com/eza-community/eza/issues/1636), [#1651](https://github.com/eza-community/eza/issues/1651), [#1663](https://github.com/eza-community/eza/issues/1663), [#1689](https://github.com/eza-community/eza/issues/1689), [#1691](https://github.com/eza-community/eza/issues/1691), [#1723](https://github.com/eza-community/eza/issues/1723), [#1724](https://github.com/eza-community/eza/issues/1724), [#1739](https://github.com/eza-community/eza/issues/1739), [#1779](https://github.com/eza-community/eza/issues/1779), [#1829](https://github.com/eza-community/eza/issues/1829), [#1834](https://github.com/eza-community/eza/issues/1834), [#1862](https://github.com/eza-community/eza/issues/1862), [#1877](https://github.com/eza-community/eza/issues/1877), [#1878](https://github.com/eza-community/eza/issues/1878), [#1893](https://github.com/eza-community/eza/issues/1893), [#1897](https://github.com/eza-community/eza/issues/1897).
