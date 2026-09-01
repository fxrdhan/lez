<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# Upstream Triage & Compatibility Reference

> **Lineage**: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).
> `lez` ports unmerged `eza-community/eza` work by hand; it does not synchronize directly with upstream.

---

## 1. Upstream Triage Overview

**Snapshot taken 2026-08-24 / 2026-08-25**, against 149 open upstream PRs and 286 open upstream issues.
- **PRs (149 total)**: 106 PRs were already ported to `lez`. 1 PR was ported directly ([#1924](https://github.com/eza-community/eza/pull/1924)), and the other 42 were audited and deliberately declined.
- **Issues (286 total)**: All 98 open `type: bug` and 180 `type: feature` / `chore` / `docs` issues were audited against our binary.

### Issue Audit Order

Work an unread upstream report in this order:
1. **Reproduce against our binary before reading further.** Most reports die here — fixed through another port, and issue titles can be misleading.
2. **Read the body and entire discussion, not just the title.**
3. **Drop eza's internal infrastructure issues** (winget, `deb.gierens.de`, upstream flake, trycmd on ZFS).
4. **Group the rest by theme, one PR per theme.**

---

## 2. Upstream PRs Deliberately Not Ported

| Reason | Upstream PRs |
|---|---|
| **Dependency bumps.** We maintain our own `Cargo.lock` and pins, several already ahead of upstream. | [#1543](https://github.com/eza-community/eza/pull/1543), [#1554](https://github.com/eza-community/eza/pull/1554), [#1575](https://github.com/eza-community/eza/pull/1575), [#1607](https://github.com/eza-community/eza/pull/1607), [#1659](https://github.com/eza-community/eza/pull/1659), [#1660](https://github.com/eza-community/eza/pull/1660), [#1666](https://github.com/eza-community/eza/pull/1666), [#1703](https://github.com/eza-community/eza/pull/1703), [#1745](https://github.com/eza-community/eza/pull/1745), [#1749](https://github.com/eza-community/eza/pull/1749) |
| **Infrastructure specific to eza.** Crane migration, the `deb.gierens.de` APT matrix, Miri, `cargo shear`, their RISC-V and musl release targets, their cross-build containers. | [#462](https://github.com/eza-community/eza/pull/462), [#971](https://github.com/eza-community/eza/pull/971), [#972](https://github.com/eza-community/eza/pull/972), [#1537](https://github.com/eza-community/eza/pull/1537), [#1538](https://github.com/eza-community/eza/pull/1538), [#1629](https://github.com/eza-community/eza/pull/1629), [#1753](https://github.com/eza-community/eza/pull/1753), [#1777](https://github.com/eza-community/eza/pull/1777), [#1861](https://github.com/eza-community/eza/pull/1861), [#1869](https://github.com/eza-community/eza/pull/1869), [#1890](https://github.com/eza-community/eza/pull/1890), [#1901](https://github.com/eza-community/eza/pull/1901) |
| **eza's own branding and README.** [#1713](https://github.com/eza-community/eza/pull/1713) renames leftover `exa` strings, which was done for `lez` in PR #37. | [#1625](https://github.com/eza-community/eza/pull/1625), [#1713](https://github.com/eza-community/eza/pull/1713), [#1755](https://github.com/eza-community/eza/pull/1755), [#1756](https://github.com/eza-community/eza/pull/1756), [#1914](https://github.com/eza-community/eza/pull/1914) |
| **Already covered by a port we took.** [#913](https://github.com/eza-community/eza/pull/913) ← [#925](https://github.com/eza-community/eza/pull/925) (WSL hyperlinks), [#1596](https://github.com/eza-community/eza/pull/1596) ← [#1923](https://github.com/eza-community/eza/pull/1923) (stdin), [#1838](https://github.com/eza-community/eza/pull/1838)/[#1840](https://github.com/eza-community/eza/pull/1840)/[#1844](https://github.com/eza-community/eza/pull/1844) ← [#1848](https://github.com/eza-community/eza/pull/1848) (all three are the same non-UTF-8 `--time-style` fix), [#1233](https://github.com/eza-community/eza/pull/1233)/[#1504](https://github.com/eza-community/eza/pull/1504) ← commit `cfe0abb7`, which removed the Windows `_`-prefix filter outright. | [#913](https://github.com/eza-community/eza/pull/913), [#1233](https://github.com/eza-community/eza/pull/1233), [#1504](https://github.com/eza-community/eza/pull/1504), [#1596](https://github.com/eza-community/eza/pull/1596), [#1838](https://github.com/eza-community/eza/pull/1838), [#1840](https://github.com/eza-community/eza/pull/1840), [#1844](https://github.com/eza-community/eza/pull/1844) |
| **Dead or disproportionate.** [#974](https://github.com/eza-community/eza/pull/974) is ±25k lines of churn under `CHANGES_REQUESTED`; [#1765](https://github.com/eza-community/eza/pull/1765) touches 744 files; [#936](https://github.com/eza-community/eza/pull/936) fixes clippy warnings we do not have; [#575](https://github.com/eza-community/eza/pull/575) waits on an upstream design decision that never came; [#1658](https://github.com/eza-community/eza/pull/1658) conflicts. | [#575](https://github.com/eza-community/eza/pull/575), [#936](https://github.com/eza-community/eza/pull/936), [#974](https://github.com/eza-community/eza/pull/974), [#1658](https://github.com/eza-community/eza/pull/1658), [#1765](https://github.com/eza-community/eza/pull/1765) |
| **Product decisions, not oversights.** [#770](https://github.com/eza-community/eza/pull/770) (`--no-header`) superseded by configuration file support (`[display] header = false`); [#1903](https://github.com/eza-community/eza/pull/1903) was solved our own way in PR #26. | [#770](https://github.com/eza-community/eza/pull/770), [#1804](https://github.com/eza-community/eza/pull/1804), [#1903](https://github.com/eza-community/eza/pull/1903) |

---

## 3. Platform Capabilities & Testing Scope

- **Linux containers on macOS**: A Linux-only report is reproducible locally in minutes — build inside `rust:*-bookworm` with `CARGO_TARGET_DIR=/tmp/target` and a mounted read-only checkout.
- **Windows testing**: Windows cannot run locally in containers; use the `Windows Repro Probe` manual GitHub workflow (`.github/workflows/windows-probe.yml`).
- **Nix flake check on macOS**: `nix flake check` does not pass on macOS due to OS differences (`nixbld` vs `_nixbld1` user, macOS `@` xattr marker). Use CI for Nix flake check validation; `nix develop` and `nix build` work fine locally.
- **Out of reach**: Specific physical USB hardware, CIFS/macFUSE mounts, systemd-homed, iSH, and terminal font glyphs.

---

## 4. Upstream Issues & Status

### Still Open Upstream (Known Status)

| Upstream ID | State & Notes |
|---|---|
| [#875](https://github.com/eza-community/eza/issues/875) | Alpine on iSH. Needs iSH; no code lead. |
| [#1088](https://github.com/eza-community/eza/issues/1088) | `LS_COLORS` `ca` (Linux capabilities) unsupported. Reproducible on Linux, not on macOS; see issue #60. |
| [#1428](https://github.com/eza-community/eza/issues/1428) | systemd-homed user names. We already resolve through `uzers` → `getpwuid_r` → NSS, which is the right API; nothing to act on without the setup. |
| [#1500](https://github.com/eza-community/eza/issues/1500) | Hang on one USB device. Needs that device. |
| [#1548](https://github.com/eza-community/eza/issues/1548) | Control-character quoting. Wants a `--quote-style` option: a design decision, not a fix. |
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
| [#1642](https://github.com/eza-community/eza/issues/1642), [#1640](https://github.com/eza-community/eza/issues/1640) | `--blocks` added for integer filesystem block count column alongside `-S` / `--blocksize` (byte size) and `bl` in `LS_COLORS` styled with block palette. |
| [#1904](https://github.com/eza-community/eza/issues/1904) | `Dir::contains` memoises into a set; 5000 `.log` files list in 0.07s |
| [#1912](https://github.com/eza-community/eza/pull/1912) | `palette_derive` pinned to `=0.7.5` beside `palette` |
| [#223](https://github.com/eza-community/eza/issues/223), [#579](https://github.com/eza-community/eza/issues/579) | the Git column is dropped when nothing in the listing is in a repo |
| [#139](https://github.com/eza-community/eza/issues/139), [#766](https://github.com/eza-community/eza/issues/766), [#770](https://github.com/eza-community/eza/pull/770), [#812](https://github.com/eza-community/eza/issues/812), [#1587](https://github.com/eza-community/eza/issues/1587), [#1707](https://github.com/eza-community/eza/issues/1707), [#1875](https://github.com/eza-community/eza/issues/1875) | Configuration file for option defaults (`config.toml`, `.lez.toml`), `--config`, `--no-config`, `LEZ_CONFIG_FILE` |

*Partly delivered*: [#584](https://github.com/eza-community/eza/issues/584) (`--quotes` yes, `-N` and `QUOTING_STYLE` no), [#600](https://github.com/eza-community/eza/issues/600) (`.tar` yes, `.zip` no), [#1466](https://github.com/eza-community/eza/issues/1466) (`--context` yes, MCS translation no), [#1735](https://github.com/eza-community/eza/issues/1735) (csv/sqlite yes, parquet/hdf5/npy no), [#1768](https://github.com/eza-community/eza/issues/1768) (`--show-dotfiles` yes, the other two axes no), [#1823](https://github.com/eza-community/eza/issues/1823) (`--git-glyphs` yes, choosing the glyph no).

### Reproduced and Fixed in `lez`

| Upstream ID | What was fixed |
|---|---|
| [#1571](https://github.com/eza-community/eza/issues/1571) | `--flags` / `-O` extended to Linux to read inode flags/attributes (`FS_IOC_GETFLAGS` / `lsattr`) in long and short format. |
| [#509](https://github.com/eza-community/eza/issues/509), [#1743](https://github.com/eza-community/eza/issues/1743), [#1448](https://github.com/eza-community/eza/issues/1448), [#1892](https://github.com/eza-community/eza/issues/1892) | `natord` was the only comparator, breaking `LC_ALL=C` hex sort. `--sort=lexicographic` added for plain comparison. |
| [#1868](https://github.com/eza-community/eza/issues/1868) | `--code` skipped dotfiles and `-a` did nothing. Fixed to count correctly with `--loc`. |
| [#922](https://github.com/eza-community/eza/issues/922), [#558](https://github.com/eza-community/eza/issues/558) | Syscall overhead (buffered stdout write and unnecessary stat when colors were off). Reduced 5000 stats to 1 on `lez -1 dir \| wc -l`. |
| [#1642](https://github.com/eza-community/eza/issues/1642), [#1640](https://github.com/eza-community/eza/issues/1640) | `--blocks` CLI flag added for integer filesystem block count column alongside `-S` / `--blocksize` (byte size) and `bl` in `LS_COLORS` styled. |
| [#765](https://github.com/eza-community/eza/issues/765) | `mh` accepted and properly parsed. |
| [#1002](https://github.com/eza-community/eza/issues/1002), [#745](https://github.com/eza-community/eza/issues/745) | High stat overhead on empty directory glyph probing over FUSE/NFS. `LEZ_NO_EMPTY_DIR_ICON` avoids probing. |
| [#1732](https://github.com/eza-community/eza/issues/1732) | `--size-digits=<NUM>` (alias `--digits`) and `LEZ_SIZE_DIGITS` added to customize size column precision/digit count. |
| [#728](https://github.com/eza-community/eza/issues/728), [#730](https://github.com/eza-community/eza/pull/730), [#1791](https://github.com/eza-community/eza/pull/1791) | Cohesive full-path quoting (`'/path/with spaces/file.txt'`) and configurable quote styling via `qu` code in `LEZ_COLORS` & `theme.yml`. |

### Reproduced but Still Open / By Design

| Upstream ID | Description & Status |
|---|---|
| [#1498](https://github.com/eza-community/eza/issues/1498) | `--total-size` walks hidden directories regardless of `--all`. Arguably expected since directory total size includes hidden files. |
| [#1919](https://github.com/eza-community/eza/issues/1919) | `.m` renders as C icon (shared with Objective-C / MATLAB; no MATLAB glyph in Nerd Fonts). |

### Specifically Audited & Declined Issues (Detailed Rationale)

| Upstream ID | Description & Proof for Declining |
|---|---|
| [#693](https://github.com/eza-community/eza/issues/693) | **`--hyperlink` "eating characters" when saved to shell variable.** Not a bug in `lez`. Uses standard GNU ls (coreutils 9.11) OSC 8 terminator (`ESC \`: `^[]8;;file://…/1^[\1^[]8;;^[\`). The character eating occurs when `echo` interprets `\a`, `\b`, `\t` from combining the terminator with the leading letter of the filename. Changing to BEL would deviate from GNU ls standard. |
| [#1360](https://github.com/eza-community/eza/issues/1360) | **Subdirectory `.gitignore` behavior.** In `lez`, explicit target arguments always display their contents (`tests/gitignore_explicit_target_tests.rs` locks in "explicit arguments override filters"). Proposed `--no-git-ignore` was also declined upstream. |
| [#519](https://github.com/eza-community/eza/issues/519) | **96G vs 103G.** By design, not an oversight. `lez` defaults to SI units ($1000^3$), while `--binary` gives 96Gi ($1024^3$). Matches upstream standard and documentation. |
| [#1548](https://github.com/eza-community/eza/issues/1548) | **Control-character quoting not shell-compatible.** Proposing a `--quote-style eza\|posix` flag is a product/design decision, not an unhandled defect. (Distinct from [#1482](https://github.com/eza-community/eza/issues/1482) which was fixed in PR #52). |

### General Declined / Not Applicable Issues

- **Upstream Packaging & Infrastructure**: [#347](https://github.com/eza-community/eza/issues/347), [#475](https://github.com/eza-community/eza/issues/475), [#601](https://github.com/eza-community/eza/issues/601), [#646](https://github.com/eza-community/eza/issues/646), [#919](https://github.com/eza-community/eza/issues/919), [#930](https://github.com/eza-community/eza/issues/930), [#951](https://github.com/eza-community/eza/issues/951), [#1033](https://github.com/eza-community/eza/issues/1033), [#1040](https://github.com/eza-community/eza/issues/1040), [#1372](https://github.com/eza-community/eza/issues/1372), [#1561](https://github.com/eza-community/eza/issues/1561), [#1605](https://github.com/eza-community/eza/issues/1605), [#1610](https://github.com/eza-community/eza/issues/1610), [#1621](https://github.com/eza-community/eza/issues/1621), [#1670](https://github.com/eza-community/eza/issues/1670), [#1748](https://github.com/eza-community/eza/issues/1748), [#1776](https://github.com/eza-community/eza/issues/1776), [#1876](https://github.com/eza-community/eza/issues/1876), doc issues [#969](https://github.com/eza-community/eza/issues/969), [#1099](https://github.com/eza-community/eza/issues/1099), [#1100](https://github.com/eza-community/eza/issues/1100), [#1487](https://github.com/eza-community/eza/issues/1487), governance [#1872](https://github.com/eza-community/eza/issues/1872).
- **Declined based on discussion**: [#479](https://github.com/eza-community/eza/issues/479) (PGO speedup minimal), [#756](https://github.com/eza-community/eza/issues/756) (reading file contents declined), [#729](https://github.com/eza-community/eza/issues/729) (`lib.rs` is internal), [#1117](https://github.com/eza-community/eza/issues/1117) (Unicode glyph rendering issues), [#905](https://github.com/eza-community/eza/issues/905)/[#1401](https://github.com/eza-community/eza/issues/1401) (blocked on Windows ACL and libgit2 sha256).

