<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# Upstream Triage & Compatibility Reference

> **Lineage**: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).
> `lez` ports unmerged `eza-community/eza` work by hand; it does not synchronize directly with upstream.

---

## 1. Upstream Triage Overview

**Last refreshed 2026-11-24**, against 199 open upstream PRs and 295 open upstream issues. The previous refresh, on 2026-10-05, counted 173 and 291. The full sweep behind most of this file was taken on 2026-08-24 and 2026-08-25, against 149 open PRs and 286 open issues.

**Upstream has not moved since the fork.** `lez` branched from upstream `main` at `471bfbc7` (eza v0.23.5, 2026-07-09). As of 2026-11-24, upstream `main` has no commit after it, there is no release after v0.23.5, no PR has been merged since 2026-08-20, and no maintainer has commented since then. Outside contributors still open PRs and issues: 28 PRs and 5 issues since 2026-10-05. Everything upstream has merged is already in `lez`; what is left to consider is open PRs and issues.

- **PRs (199 open)**: 42 were declined in the 2026-08-24 sweep (§2). 106 had already been ported by then, and [#1924](https://github.com/eza-community/eza/pull/1924) was ported in it. Each `lez` PR body links the upstream PRs it ports, and `CHANGELOG.md` links them per release. The other 24 were opened after the sweep, or missed by it, and are triaged in §3. Since 2026-10-05, [#1804](https://github.com/eza-community/eza/pull/1804) and [#1884](https://github.com/eza-community/eza/pull/1884) were closed and 28 more were opened; those 28 are triaged in §3 too.
- **Issues (295 open)**: the 2026-08-24 sweep read every open bug report, and the 2026-08-25 sweep every other open issue. What they found is in §5, together with the reports re-run against v0.28.5 on 2026-10-05. Their verdicts on the issues they did not name were never written into this repository, so §5 ends with a list of those issues. The 23 bug reports on that list were run against v0.29.1 on 2026-11-18 and are recorded in §5. The 2026-11-24 refresh found eight more unrecorded: the five opened since 2026-10-05, and three older bug reports, [#825](https://github.com/eza-community/eza/issues/825), [#1468](https://github.com/eza-community/eza/issues/1468) and [#1715](https://github.com/eza-community/eza/issues/1715), that the earlier lists missed. All eight are in §5. [#1941](https://github.com/eza-community/eza/issues/1941) was closed as a duplicate. Of the 70 feature requests, five now have an upstream PR and are under *Pending Decisions*; the other 65 are still undecided.

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

Declined in the 2026-08-24 sweep. All 42 were still open on 2026-10-05, and each reason still holds. [#1804](https://github.com/eza-community/eza/pull/1804) was closed upstream on 2026-10-09; the other 41 were still open on 2026-11-24. PRs opened since are triaged in §3.

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
| [#1949](https://github.com/eza-community/eza/pull/1949) `--finder-meta` | **Partly covered.** `-e`/`--tags` lists Finder colour tags beside the name; colouring the name itself is pending with #1974 below, and the SF Symbol overlays are declined (§5). |
| [#1950](https://github.com/eza-community/eza/pull/1950) trailing separators on Windows ([#404](https://github.com/eza-community/eza/issues/404)) | **Covered.** #404 was verified fixed on a `windows-latest` runner in issue #57. |
| [#1951](https://github.com/eza-community/eza/pull/1951) no panic on a closed stdout pipe on Windows | **Not yet checked.** Needs the Windows probe (§4). |
| [#1952](https://github.com/eza-community/eza/pull/1952) bounded `listxattr` retries on macOS ([#1850](https://github.com/eza-community/eza/issues/1850)) | **Covered.** Commit `7a3487d7` (PR #38) caps the `ERANGE` retry loop. The macFUSE hang in #1850 itself is out of reach (§4). |
| [#1953](https://github.com/eza-community/eza/pull/1953) wildcard expansion on Windows ([#337](https://github.com/eza-community/eza/issues/337)) | **Covered.** PR #61. |
| [#1955](https://github.com/eza-community/eza/pull/1955) `--total-size -aa` skips `..` ([#1690](https://github.com/eza-community/eza/issues/1690)) | **Covered.** `lez -aal --total-size` prints `-` as the size of `..`. |
| [#1956](https://github.com/eza-community/eza/pull/1956) `.tzst` extension | **Ported**, with tests, by `feat(icons): treat .tzst as a compressed tarball`. |

### Opened Since 2026-10-05

Triaged 2026-11-24, against v0.30.0 on macOS. A PR marked *Pending decision* adds something `lez` does not have; what it would take is under *Pending Decisions* in §5.

| Upstream PR | Verdict |
|---|---|
| [#1957](https://github.com/eza-community/eza/pull/1957), [#1995](https://github.com/eza-community/eza/pull/1995) `--recurse --level` counted from the listed directory ([#1701](https://github.com/eza-community/eza/issues/1701), [#1991](https://github.com/eza-community/eza/issues/1991)) | **Covered.** See #1991 in §5. |
| [#1958](https://github.com/eza-community/eza/pull/1958) escape C1 control characters ([#1444](https://github.com/eza-community/eza/issues/1444)) | **Covered.** See #1444 in §5. |
| [#1960](https://github.com/eza-community/eza/pull/1960) MATLAB icon for `.m` ([#1919](https://github.com/eza-community/eza/issues/1919)) | **Pending decision.** |
| [#1961](https://github.com/eza-community/eza/pull/1961) Simplified Chinese documentation | **Declined.** Translations of eza's own documents. |
| [#1962](https://github.com/eza-community/eza/pull/1962) wildcard arguments on Windows ([#337](https://github.com/eza-community/eza/issues/337)) | **Covered.** PR #61, as for #1953 above. |
| [#1963](https://github.com/eza-community/eza/pull/1963) `LS_COLORS` `tw`, `ow` and `st` for directories | **Pending decision.** `lez` reads none of the three from `LS_COLORS` (`ls_unsupported_indicators` in `src/theme/mod.rs`). |
| [#1964](https://github.com/eza-community/eza/pull/1964) `palette` 0.7.7 without the exact pin | **Declined.** A dependency bump. The mismatch it fixes cannot occur here, since `palette_derive` is pinned to `=0.7.5` beside `palette` (#1912 in §5). |
| [#1965](https://github.com/eza-community/eza/pull/1965) `--version` fixtures for v0.23.5 | **Declined.** eza's own snapshots. |
| [#1966](https://github.com/eza-community/eza/pull/1966) about 200 more directory icons | **Pending decision.** `DIRECTORY_ICONS` has 45 entries. |
| [#1970](https://github.com/eza-community/eza/pull/1970) `nix` and `Developer` directory icons ([#1636](https://github.com/eza-community/eza/issues/1636), [#1663](https://github.com/eza-community/eza/issues/1663)) | **Pending decision.** Neither name is in `DIRECTORY_ICONS`. |
| [#1971](https://github.com/eza-community/eza/pull/1971) `--include-glob` ([#1471](https://github.com/eza-community/eza/issues/1471)) | **Pending decision.** |
| [#1972](https://github.com/eza-community/eza/pull/1972) `--one-file-system` ([#1689](https://github.com/eza-community/eza/issues/1689)) | **Pending decision.** |
| [#1973](https://github.com/eza-community/eza/pull/1973) keep symlinked directories under `--only-dirs --follow-symlinks` ([#1873](https://github.com/eza-community/eza/issues/1873)) | **Declined.** `--only-dirs` leaves symlinks out and `--show-symlinks` brings them back: in a tree holding `Folder/link -> ../real`, `-T --follow-symlinks --only-dirs` lists `Folder` without the link, and adding `--show-symlinks` lists the link and `inner` under it. See #1873 in §5. |
| [#1974](https://github.com/eza-community/eza/pull/1974) colour file names by Finder tag in long listings | **Pending decision.** Narrowed on 2026-11-24 (commit `eb59dd1`) after our comment: the SF Symbol overlays from #1949 are gone, and `-l --finder-meta` colours a name by its first coloured tag. `lez` has the tags but not the colouring: `-l -e` shows a folder tagged `红色\n6` with a red `红色` tag beside a name still in the directory style (`1;36`), and decodes a 31-character tag name as well. |
| [#1975](https://github.com/eza-community/eza/pull/1975) `--no-git-ignore` ([#1587](https://github.com/eza-community/eza/issues/1587)) | **Pending decision.** #1587 reproduces: in a repository whose `.gitignore` is `*`, `!.gitignore` and `!kept`, as Homebrew's is, `lez --git-ignore repo` and `-T --git-ignore repo` list `kept` alone, so an alias that carries `--git-ignore` hides nearly all of `/opt/homebrew`. |
| [#1978](https://github.com/eza-community/eza/pull/1978) exit 13 when a listed path is unreadable ([#1977](https://github.com/eza-community/eza/issues/1977)) | **Covered.** See #1977 in §5. |
| [#1979](https://github.com/eza-community/eza/pull/1979) prefer metadata for file type decisions ([#1464](https://github.com/eza-community/eza/issues/1464)) | **Covered** by PR #62, which re-checks only an unusual `readdir` type with a stat, so ordinary entries make no extra syscall. |
| [#1981](https://github.com/eza-community/eza/pull/1981) `EZA_SIZE_STYLE` ([#657](https://github.com/eza-community/eza/issues/657)) | **Pending decision.** |
| [#1982](https://github.com/eza-community/eza/pull/1982) `user_root` before `user_you` when running as root ([#1788](https://github.com/eza-community/eza/issues/1788)) | **Declined.** `uu` is "a user that's you" (`lez_colors(5)`), root included when you are root, as the #1788 re-run in §5 records. |
| [#1983](https://github.com/eza-community/eza/pull/1983) non-UTF-8 `--time-style` ([#1837](https://github.com/eza-community/eza/issues/1837)) | **Covered**, as #1848 in §2: `lez` says the value is not valid UTF-8 and exits 3. |
| [#1984](https://github.com/eza-community/eza/pull/1984) icons for `.atom`, `.idea`, `.rvm` and `.zsh_sessions` directories ([#1940](https://github.com/eza-community/eza/issues/1940)) | **Covered.** Moved to `DIRECTORY_ICONS` with the #1943 port (§5). |
| [#1986](https://github.com/eza-community/eza/pull/1986) inline directories in tree mode (draft) | **Pending decision.** |
| [#1987](https://github.com/eza-community/eza/pull/1987) `libz-sys` 1.1.33 for RUSTSEC-2026-0418 | **Covered.** `Cargo.lock` already has 1.1.33. |
| [#1988](https://github.com/eza-community/eza/pull/1988) every flag combination in `filter_child_files` ([#1927](https://github.com/eza-community/eza/issues/1927)) | **Covered** by the #1930 port (PR #93): `--only-files --no-symlinks` lists the plain file alone. |
| [#1989](https://github.com/eza-community/eza/pull/1989) `windows-11-arm` in the unit test matrix | **Declined.** eza's own CI. `lez` builds `aarch64-pc-windows-msvc` in its release jobs (#1929 in §5). |
| [#1990](https://github.com/eza-community/eza/pull/1990) fish completion for `--sort=created` | **Ported.** `completions/fish/lez.fish` and its `eza` copy described `created` as "Sort by file modified time"; they now say created time, and `fish_describes_each_timestamp_sort_by_the_timestamp_it_uses` checks every timestamp sort value against the field clap maps it to. |
| [#1994](https://github.com/eza-community/eza/pull/1994) `Gr` style for directories that are Git repositories ([#1993](https://github.com/eza-community/eza/issues/1993)) | **Pending decision.** |

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
| [#139](https://github.com/eza-community/eza/issues/139), [#766](https://github.com/eza-community/eza/issues/766), [#770](https://github.com/eza-community/eza/pull/770), [#812](https://github.com/eza-community/eza/issues/812), [#1707](https://github.com/eza-community/eza/issues/1707), [#1875](https://github.com/eza-community/eza/issues/1875) | Configuration file for option defaults (`config.toml`, `.lez.toml`), `--config`, `--no-config`, `LEZ_CONFIG_FILE` |
| [#1823](https://github.com/eza-community/eza/issues/1823) | Configurable Git indicator status glyphs and repository status glyphs in `theme.yml` under `git` and `git_repo` (`glyph: "..."`) alongside `--git-glyphs`. |
| [#1466](https://github.com/eza-community/eza/issues/1466) | `--context` / `-Z` SELinux MCS translation support (`selinux_raw_to_trans_context`) on Linux. |
| [#1933](https://github.com/eza-community/eza/issues/1933) | `--no-language` flag and `[loc] language = false` configuration to suppress Language column when `--loc` is enabled. |
| [#1927](https://github.com/eza-community/eza/issues/1927), [#1930](https://github.com/eza-community/eza/pull/1930) | Independent composable file type filters (`--only-files`, `--only-dirs`, `--no-symlinks`, `--show-symlinks`), resolving filter cancellation when combining flags (PR #93). |
| [#1846](https://github.com/eza-community/eza/issues/1846), [#1931](https://github.com/eza-community/eza/pull/1931) | Repeatable options in Zsh completions (`completions/zsh/_lez` and `_eza`) via `*` prefix for all options (PR #93). |
| [#1498](https://github.com/eza-community/eza/issues/1498), [#1291](https://github.com/eza-community/eza/issues/1291) | Recursive size calculation (`--total-size`) respects `--all` / `dot_filter`, skipping hidden directories when dotfiles are not shown (commit `b4e0a3d4`, PR #38). The 2026-08-25 sweep recorded #1498 as still reproducing; that was wrong. A build of the PR #38 merge already gives `1.0k` without `-a` and `101k` with it, for a directory holding a 100 kB hidden file. |
| [#1922](https://github.com/eza-community/eza/issues/1922) | `--code` counts the lines in a Markdown fence under the fence's language (PRs #109, #111) |
| [#1929](https://github.com/eza-community/eza/issues/1929) | the release workflow builds `aarch64-pc-windows-msvc` |
| [#1776](https://github.com/eza-community/eza/issues/1776) | the release workflow builds a static `aarch64-unknown-linux-musl` binary, beside the `x86_64-unknown-linux-musl` one |
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
| [#585](https://github.com/eza-community/eza/issues/585) | A name that is not UTF-8 was printed as U+FFFD for each byte that is part of no character, into a pipe too, so nothing reading the listing could have the name back; the reporter's screenshot is that character read as ISO-8859-7 (`οΏ½`). Issue #58 had set this aside as needing a font and a terminal; it needs neither, only Linux. Each such byte is now written as itself: in octal inside ANSI-C quotes (`$'\302\337\355\364\345\357'`), which a shell reads back as the name, and as `\xc2` under `--quotes=never`. It holds for a name, a path before it, a link's target, a directory's header and `--absolute`. `lez` still does not decode the name in the locale's character set, so it does not print the Greek that `ls` prints under `el_GR.ISO-8859-7`; `--json` still gives U+FFFD, since a JSON string cannot hold the bytes; and such a name gets no `--hyperlink`. |
| [#755](https://github.com/eza-community/eza/issues/755) | `--color-scale` wrote every shade as `38;2;r;g;b` whatever the terminal, a date in `fixed` mode too, and a terminal without 24-bit colour may read those numbers as codes of their own and show colours that have nothing to do with the scale. The shades are now 24-bit only where the terminal says it takes them (`COLORTERM` of `truecolor` or `24bit`, a `TERM` ending in `-direct` or `-truecolor`, or a Windows console, which sets neither) and the nearest entries of the 256-colour palette, from 16 up, anywhere else. A session over `ssh` or through `sudo` does not carry `COLORTERM`, and gets the coarser scale until it is set there. Colours a theme or `LS_COLORS` names are written as given. The rest of the report is left: the default listing uses no 24-bit colour, nothing is scaled unless `--color-scale` asks, and a terminal with fewer than 256 colours is not catered for. |
| [#1592](https://github.com/eza-community/eza/issues/1592) | `--long --oneline` lists one name per line and `--oneline --long` gives the long view. That is by design, the later of the two wins, and `the_last_of_long_and_oneline_wins` holds it; `lez(1)` did not say so, which was the complaint. It does now, under both flags, with what `--oneline` does beside `--grid` and `--tree`. The report's other half, `--across` ignored in `--grid --long`, does not reproduce. |
| [#874](https://github.com/eza-community/eza/issues/874) | Under `cmd.exe` and Windows PowerShell 5.1, `"C:\Program Files\"` reached the program as `C:\Program Files"` and raised `os error 123`: neither shell touches the `\"`, which a program reading its command line takes for a quote inside the argument. It is what tab completion writes in PowerShell for a directory with a space in its name. Confirmed on a `windows-latest` runner, through `cmd` and PowerShell 5.1.26100; PowerShell 7 doubles the backslash, which is why the probe in issue #57 passed. A quote at the very end of a path is now put back as the separator that was typed. With anything after it on the line, as in `"C:\Program Files\" -l`, the quote takes that into the argument too and where the path ended cannot be told from it: that is refused as before, with a line saying what happened and to leave the backslash off or double it. |

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

### Re-run 2026-11-18: The Bug Reports Left Unrecorded

Run against v0.29.1 on macOS, and on Linux (`rust:1-bookworm`) where a row says so. These are the 23 bug reports the 2026-10-05 refresh found unrecorded. The 15 below do not reproduce. Of the other eight, [#585](https://github.com/eza-community/eza/issues/585), [#755](https://github.com/eza-community/eza/issues/755), [#874](https://github.com/eza-community/eza/issues/874) and [#1592](https://github.com/eza-community/eza/issues/1592) are under *Reproduced and Fixed in `lez`*, [#541](https://github.com/eza-community/eza/issues/541) and [#1489](https://github.com/eza-community/eza/issues/1489) are under *Reproduced but Still Open / By Design*, and [#1569](https://github.com/eza-community/eza/issues/1569) and [#1623](https://github.com/eza-community/eza/issues/1623) are under *General Declined / Not Applicable Issues*.

| Upstream ID | Checked with | Result |
|---|---|---|
| [#576](https://github.com/eza-community/eza/issues/576) | `--git-repos-no-status --long --header` and `--git-repos` over 25 repositories, 500 runs, 100 of them under `MallocScribble`, `MallocGuardEdges` and `MallocCheckHeapEach` | no allocator error. The reporter saw the double free once and could not repeat it, so this is not proof |
| [#682](https://github.com/eza-community/eza/issues/682) | `EZA_COLORS='sn=31:sb=31'` and `LEZ_COLORS='sn=35:sb=35'`, `-l` without `--color-scale` | the size takes the colour |
| [#827](https://github.com/eza-community/eza/issues/827) | a name holding a newline, `-l --icons=always` and `--grid --icons=always` | one row, the name as `$'with\nnewline'` |
| [#857](https://github.com/eza-community/eza/issues/857) | `--sort=time` over eight files, `--grid` at 80 columns | oldest first, read down each column, which is how `ls` fills a grid; `--across` reads along the rows. Not a defect |
| [#865](https://github.com/eza-community/eza/issues/865) | `-T -L 2 --ignore-glob 'node_modules/*'` | `node_modules` listed, its contents hidden |
| [#1054](https://github.com/eza-community/eza/issues/1054) | zsh with `completions/zsh/_lez`: `lez -F <TAB>`, and an alias that ends in `-F` | file names; `--classify=<TAB>` offers the values |
| [#1361](https://github.com/eza-community/eza/issues/1361) | `-Ta` over a tree with hidden directories and links to `.`, `..` and its own root | ends at once, no link followed. Not tried on a whole home directory, which is what the reporter listed |
| [#1444](https://github.com/eza-community/eza/issues/1444) | `a`, `b\302\237` and `c` in one directory, grid and `-1` | all three listed, the second as `$'b\302\237'` |
| [#1578](https://github.com/eza-community/eza/issues/1578) | theme with `extensions: nix: {icon: {glyph: "❄"}}` beside a `filenames: Desktop` entry | `flake.nix` takes the glyph and every other icon stays. A `Desktop` directory keeps its folder icon, as in #1116 |
| [#1648](https://github.com/eza-community/eza/issues/1648) | `-l --smart-group` on a file whose group is not its owner's | the group is shown without `--group` |
| [#1674](https://github.com/eza-community/eza/issues/1674) | `TZ=Europe/Berlin`, files dated midnight on 2010-12-14 and 2014-05-20, on macOS and on Linux | `14 Dec 2010` and `20 May 2014`, as `ls` gives |
| [#1706](https://github.com/eza-community/eza/issues/1706) | `git init`, `git add README.md`, `-l --git` | `N-`, which `lez(1)` gives as a new file, staged. Status comes from libgit2, so no `git` process is expected. Not tried under WSL |
| [#1781](https://github.com/eza-community/eza/issues/1781) | `-l --color-scale=age -T -L 2`, with and without `--total-size --git` | each row takes the colour of its own age, at both levels |
| [#1788](https://github.com/eza-community/eza/issues/1788) | the reporter's `users:` theme, as root and as another user, on Linux | as root, other users take `user_other` and root takes `user_you` |
| [#1873](https://github.com/eza-community/eza/issues/1873) | the reporter's tree, `--tree --follow-symlinks --only-dirs ./Folder-Link` | nothing: `--only-dirs` leaves out symlinks, the argument included. With `--show-symlinks` the link and both its children are listed. The half-followed listing does not occur |

### Re-run 2026-11-24: Reports Opened or Missed Since 2026-10-05

Run against v0.30.0 on macOS, as a user who is not root. These are the four bug reports among the eight issues the 2026-11-24 refresh found unrecorded, and none reproduces. Of the other four, [#1959](https://github.com/eza-community/eza/issues/1959) and [#1993](https://github.com/eza-community/eza/issues/1993) are under *Pending Decisions*, and [#825](https://github.com/eza-community/eza/issues/825) and [#1976](https://github.com/eza-community/eza/issues/1976) under *General Declined / Not Applicable Issues*.

| Upstream ID | Checked with | Result |
|---|---|---|
| [#1468](https://github.com/eza-community/eza/issues/1468) | `--hyperlink=always` on `h[1].txt` | the link ends in `h%5B1%5D.txt`, brackets percent-encoded |
| [#1715](https://github.com/eza-community/eza/issues/1715) | `-1 -F=always --group-directories-first` over `src/`, `a.mp3`, `z.yml` and a broken link `broken` | `src/`, `a.mp3`, `broken@`, `z.yml`: the broken link sorts with the files |
| [#1977](https://github.com/eza-community/eza/issues/1977) | a mode `000` directory listed directly and with `-l`, and its parent with `-R` and with `-T` | exit 13 each time |
| [#1991](https://github.com/eza-community/eza/issues/1991) | the reporter's tree, `./test -R -L 1` to `-L 3`, and `-L 2` with an absolute path | the levels the reporter expected: `-L 2` lists `ABC` and the contents of `test/ABC`, from a relative path and from an absolute one |

### Reproduced but Still Open / By Design

| Upstream ID | Description & Status |
|---|---|
| [#541](https://github.com/eza-community/eza/issues/541) | By design. A static musl binary cannot load the system's NSS modules, so it reads names from `/etc/passwd` and `/etc/group` alone. On Linux, with a user known only through `libnss-extrausers`, the glibc build lists `ldapuser g0620` and the musl build `31128555 20620`. The release carries both builds, the installer takes the musl one only where the C library is musl, and `INSTALL.md` says which to prefer. |
| [#1489](https://github.com/eza-community/eza/issues/1489) | By design. A symlink's arrow is bright black (`1;90`) and temporary files such as `.bak` are dim (`2`), which some dark themes hide. `xx` and `tm` in `LEZ_COLORS` change them. |

### Pending Decisions

Real requests that `lez` has not answered either way, each needing a product decision before any code. The seven open on 2026-10-05 were settled that day, and their outcomes are in the tables above and below. The twelve below were added on 2026-11-24: each has an upstream PR or issue from after 2026-10-05, and the last column gives what `lez` would pay for it under the performance rules in `AGENTS.md`.

| Upstream | Request | What it would take in `lez` |
|---|---|---|
| [#1919](https://github.com/eza-community/eza/issues/1919), [#1960](https://github.com/eza-community/eza/pull/1960) | `.m` shown as MATLAB rather than C | One table entry. Nerd Fonts has a MATLAB glyph, `dev-matlab` at U+E82A (checked in JetBrainsMono Nerd Font), so the reason once given for keeping the C icon is gone. Icons go by extension alone, so Objective-C files would show the MATLAB glyph. |
| [#1963](https://github.com/eza-community/eza/pull/1963) | `tw`, `ow` and `st` in `LS_COLORS` style directories by their sticky and others-writable bits, as in GNU `ls` | The mode bits, so a stat for each directory in a short view, only when one of the keys is set. In `LEZ_COLORS`, `tw` already means the others-write bit of the permissions column. |
| [#1993](https://github.com/eza-community/eza/issues/1993), [#1994](https://github.com/eza-community/eza/pull/1994) | a style for directories that hold a `.git` entry | One `lstat` of `dir/.git` per directory, only when the style is set; no libgit2. |
| [#1471](https://github.com/eza-community/eza/issues/1471), [#1971](https://github.com/eza-community/eza/pull/1971) | `--include-glob`, listing only names that match | A name match in memory, no syscall. The PR keeps every directory so that `-T` can descend, as `tree -P` does. |
| [#1689](https://github.com/eza-community/eza/issues/1689), [#1972](https://github.com/eza-community/eza/pull/1972) | `--one-file-system`: `-R` and `-T` do not enter a directory on another filesystem | Each directory's device ID compared with its parent's, in the one traversal engine. The PR does nothing on Windows, where `std` has no stable device ID. |
| [#657](https://github.com/eza-community/eza/issues/657), [#1981](https://github.com/eza-community/eza/pull/1981) | the default size style (binary, decimal or bytes) from an environment variable | A variable read in the order `LEZ_*`, then `EZA_*`, and a `[display]` key beside it. Making binary the built-in default is a different question; see #519 below. |
| [#1636](https://github.com/eza-community/eza/issues/1636), [#1663](https://github.com/eza-community/eza/issues/1663), [#1970](https://github.com/eza-community/eza/pull/1970) | `nix` and `Developer` directory icons | Two table entries. `nix` would also give `/nix` the icon. |
| [#1966](https://github.com/eza-community/eza/pull/1966) | about 200 more directory icons: game and user folders, `bin`, `lib`, `tests`, tool dot-directories | Table entries. The question is which generic names should carry an icon everywhere. |
| [#1986](https://github.com/eza-community/eza/pull/1986) (draft) | a chain of single-child directories shown as one tree row (`src/test/java/org`) | A change to the tree renderer. At the `--level` cut-off it would have to read one level further to know whether a directory has a single child. |
| [#1587](https://github.com/eza-community/eza/issues/1587), [#1975](https://github.com/eza-community/eza/pull/1975) | `--no-git-ignore=PATH`, exempting some paths from `--git-ignore` | A path comparison per directory, no syscall. Whether the exemption covers everything below `PATH`, and how it meets `.gitignore` files inside it, is undecided. Today's answer is partial: with `git_ignore = true` in the global configuration, a `.lez.toml` holding `git_ignore = false` turns it off in that directory, but only while it is the working directory; listed from elsewhere, its contents are still hidden. |
| [#1959](https://github.com/eza-community/eza/issues/1959) | sorting by the value of an extended attribute | A `getxattr` per file, only under that sort. The request does not say which attribute, or where files without it go. |
| [#1974](https://github.com/eza-community/eza/pull/1974) | on macOS, a file name coloured by its first coloured Finder tag in the long view | Reading the tag attribute of every file in the long view, only when asked for. The PR reads every attribute's name and value for it (`extended_attributes()`), where `lez`'s `@` marker only asks `listxattr` for a size (`has_attributes` in `src/fs/feature/xattr.rs`). The cost has not been measured. |

### Specifically Audited & Declined Issues (Detailed Rationale)

| Upstream ID | Description & Proof for Declining |
|---|---|
| [#693](https://github.com/eza-community/eza/issues/693) | **`--hyperlink` "eating characters" when saved to shell variable.** Not a bug in `lez`. It ends OSC 8 with ST (`ESC \`: `^[]8;;file://…/1^[\1^[]8;;^[\`), as GNU `ls` has since coreutils 9.10 (February 2026); 9.9 and earlier end it with BEL. The character eating occurs when `echo` interprets `\a`, `\b`, `\t` from combining the terminator with the leading letter of the filename. Changing to BEL would deviate from GNU ls standard. |
| [#1360](https://github.com/eza-community/eza/issues/1360) | **Subdirectory `.gitignore` behavior.** In `lez`, explicit target arguments always display their contents (`tests/git/gitignore.rs` locks in "explicit arguments override filters"). Proposed `--no-git-ignore` was also declined upstream. |
| [#519](https://github.com/eza-community/eza/issues/519) | **96G vs 103G.** By design, not an oversight. `lez` defaults to SI units ($1000^3$), while `--binary` gives 96Gi ($1024^3$). Matches upstream standard and documentation. |
| [#1949](https://github.com/eza-community/eza/pull/1949) | **SF Symbol overlays for Finder metadata on macOS (`--finder-meta`).** The PR draws a folder's SF Symbol as one of about 230 emoji picked by hand to look like it, a table that would have to follow each macOS release and that cannot be checked from here. Finder's colour tags, the other half of the PR, are listed by `-e`/`--tags` beside the name; colouring the name by its tag, which #1974 keeps on its own, is under *Pending Decisions*. |

### General Declined / Not Applicable Issues

- **Upstream Packaging & Infrastructure**: [#347](https://github.com/eza-community/eza/issues/347), [#475](https://github.com/eza-community/eza/issues/475), [#601](https://github.com/eza-community/eza/issues/601), [#613](https://github.com/eza-community/eza/issues/613) (trycmd on ZFS), [#646](https://github.com/eza-community/eza/issues/646), [#919](https://github.com/eza-community/eza/issues/919), [#930](https://github.com/eza-community/eza/issues/930), [#951](https://github.com/eza-community/eza/issues/951), [#973](https://github.com/eza-community/eza/issues/973) (their `idump`), [#985](https://github.com/eza-community/eza/issues/985) (`deb.gierens.de` build), [#1033](https://github.com/eza-community/eza/issues/1033), [#1040](https://github.com/eza-community/eza/issues/1040), [#1185](https://github.com/eza-community/eza/issues/1185) (winget), [#1372](https://github.com/eza-community/eza/issues/1372), [#1561](https://github.com/eza-community/eza/issues/1561), [#1569](https://github.com/eza-community/eza/issues/1569) (apt mirror errors, nothing of eza's), [#1605](https://github.com/eza-community/eza/issues/1605), [#1610](https://github.com/eza-community/eza/issues/1610), [#1619](https://github.com/eza-community/eza/issues/1619), [#1622](https://github.com/eza-community/eza/issues/1622), [#1623](https://github.com/eza-community/eza/issues/1623) (a third-party installer finding no macOS asset; `lez` releases carry both macOS archives), [#1871](https://github.com/eza-community/eza/issues/1871) (their flake on Darwin), [#1621](https://github.com/eza-community/eza/issues/1621), [#1670](https://github.com/eza-community/eza/issues/1670), [#1748](https://github.com/eza-community/eza/issues/1748), [#1876](https://github.com/eza-community/eza/issues/1876), [#1888](https://github.com/eza-community/eza/issues/1888) (their `--version` snapshot), doc issues [#969](https://github.com/eza-community/eza/issues/969), [#1099](https://github.com/eza-community/eza/issues/1099), [#1100](https://github.com/eza-community/eza/issues/1100), [#1487](https://github.com/eza-community/eza/issues/1487), governance [#1872](https://github.com/eza-community/eza/issues/1872).
- **Outside what `lez` can act on**: [#825](https://github.com/eza-community/eza/issues/825) (icons missing in the JetBrains console with one Nerd Font, a terminal's font rendering, out of reach per §4), [#1976](https://github.com/eza-community/eza/issues/1976) (a jailbroken iPhone or iPad, for which `lez` builds no binary).
- **Declined based on discussion**: [#479](https://github.com/eza-community/eza/issues/479) (PGO speedup minimal), [#756](https://github.com/eza-community/eza/issues/756) (reading file contents declined), [#729](https://github.com/eza-community/eza/issues/729) (`lib.rs` is internal), [#1117](https://github.com/eza-community/eza/issues/1117) (Unicode glyph rendering issues), [#905](https://github.com/eza-community/eza/issues/905)/[#1401](https://github.com/eza-community/eza/issues/1401) (blocked on Windows ACL and libgit2 sha256).

### Not Recorded Here

On 2026-10-05, 93 open upstream issues were named neither here nor in a `lez` PR body or `CHANGELOG.md`. The 23 bug reports among them were run on 2026-11-18 and are recorded above. The eight more found on 2026-11-24 are recorded above too. That leaves:

- **65 feature requests.** The 2026-08-25 sweep read every one and concluded that a request it did not name is a real gap and a product decision, not an oversight. Its notes on each were not kept, so treat these as undecided rather than declined. Five more, [#657](https://github.com/eza-community/eza/issues/657), [#1471](https://github.com/eza-community/eza/issues/1471), [#1636](https://github.com/eza-community/eza/issues/1636), [#1663](https://github.com/eza-community/eza/issues/1663) and [#1689](https://github.com/eza-community/eza/issues/1689), moved to *Pending Decisions* on 2026-11-24, since upstream PRs opened after 2026-10-05 implement them. The 65: [#148](https://github.com/eza-community/eza/issues/148), [#242](https://github.com/eza-community/eza/issues/242), [#292](https://github.com/eza-community/eza/issues/292), [#309](https://github.com/eza-community/eza/issues/309), [#315](https://github.com/eza-community/eza/issues/315), [#372](https://github.com/eza-community/eza/issues/372), [#389](https://github.com/eza-community/eza/issues/389), [#559](https://github.com/eza-community/eza/issues/559), [#568](https://github.com/eza-community/eza/issues/568), [#574](https://github.com/eza-community/eza/issues/574), [#593](https://github.com/eza-community/eza/issues/593), [#647](https://github.com/eza-community/eza/issues/647), [#650](https://github.com/eza-community/eza/issues/650), [#660](https://github.com/eza-community/eza/issues/660), [#683](https://github.com/eza-community/eza/issues/683), [#700](https://github.com/eza-community/eza/issues/700), [#731](https://github.com/eza-community/eza/issues/731), [#738](https://github.com/eza-community/eza/issues/738), [#751](https://github.com/eza-community/eza/issues/751), [#792](https://github.com/eza-community/eza/issues/792), [#807](https://github.com/eza-community/eza/issues/807), [#851](https://github.com/eza-community/eza/issues/851), [#854](https://github.com/eza-community/eza/issues/854), [#876](https://github.com/eza-community/eza/issues/876), [#939](https://github.com/eza-community/eza/issues/939), [#943](https://github.com/eza-community/eza/issues/943), [#976](https://github.com/eza-community/eza/issues/976), [#991](https://github.com/eza-community/eza/issues/991), [#997](https://github.com/eza-community/eza/issues/997), [#1019](https://github.com/eza-community/eza/issues/1019), [#1089](https://github.com/eza-community/eza/issues/1089), [#1094](https://github.com/eza-community/eza/issues/1094), [#1103](https://github.com/eza-community/eza/issues/1103), [#1134](https://github.com/eza-community/eza/issues/1134), [#1174](https://github.com/eza-community/eza/issues/1174), [#1213](https://github.com/eza-community/eza/issues/1213), [#1223](https://github.com/eza-community/eza/issues/1223), [#1242](https://github.com/eza-community/eza/issues/1242), [#1250](https://github.com/eza-community/eza/issues/1250), [#1329](https://github.com/eza-community/eza/issues/1329), [#1350](https://github.com/eza-community/eza/issues/1350), [#1354](https://github.com/eza-community/eza/issues/1354), [#1369](https://github.com/eza-community/eza/issues/1369), [#1385](https://github.com/eza-community/eza/issues/1385), [#1416](https://github.com/eza-community/eza/issues/1416), [#1433](https://github.com/eza-community/eza/issues/1433), [#1441](https://github.com/eza-community/eza/issues/1441), [#1494](https://github.com/eza-community/eza/issues/1494), [#1523](https://github.com/eza-community/eza/issues/1523), [#1547](https://github.com/eza-community/eza/issues/1547), [#1584](https://github.com/eza-community/eza/issues/1584), [#1611](https://github.com/eza-community/eza/issues/1611), [#1651](https://github.com/eza-community/eza/issues/1651), [#1691](https://github.com/eza-community/eza/issues/1691), [#1723](https://github.com/eza-community/eza/issues/1723), [#1724](https://github.com/eza-community/eza/issues/1724), [#1739](https://github.com/eza-community/eza/issues/1739), [#1779](https://github.com/eza-community/eza/issues/1779), [#1829](https://github.com/eza-community/eza/issues/1829), [#1834](https://github.com/eza-community/eza/issues/1834), [#1862](https://github.com/eza-community/eza/issues/1862), [#1877](https://github.com/eza-community/eza/issues/1877), [#1878](https://github.com/eza-community/eza/issues/1878), [#1893](https://github.com/eza-community/eza/issues/1893), [#1897](https://github.com/eza-community/eza/issues/1897).
