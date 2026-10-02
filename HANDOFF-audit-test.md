# Serah terima: audit test lez (cloud → lokal)

Ditulis 2026-10-02. Semua pekerjaan sudah di-commit dan di-push. Tidak ada yang tertinggal di container cloud kecuali file ini dan cache build.

## 1. Membawa ke lokal

- Branch: `c/practical-thompson-mhcnw3`
- HEAD: `ff3dc65`, yaitu 55 commit di atas `origin/dev` (`b6dd399`).
- Belum ada PR untuk branch ini. PR #148 berstatus closed dan isinya sudah ada di `dev`.

Kalau belum punya branch ini di lokal:

```sh
git fetch origin
git switch -c c/practical-thompson-mhcnw3 --track origin/c/practical-thompson-mhcnw3
```

Kalau sudah punya: history-nya di-rewrite (autosquash + `--force-with-lease`), jadi samakan dengan remote. Perintah ini membuang perubahan lokal yang belum di-commit di branch itu:

```sh
git fetch origin
git switch c/practical-thompson-mhcnw3
git reset --hard origin/c/practical-thompson-mhcnw3
```

Verifikasi cepat:

```sh
cargo nextest run --workspace   # nextest 0.9.128 (MSRV 1.90)
cargo test --doc
```

Di macOS, dua test berikut di-skip kecuali ada setcap/sudo: `linux_capabilities` dan `ls_colors_caps`. Keduanya Linux-only, jadi otomatis tidak jalan di macOS.

## 2. Status saat serah terima

Hasil terverifikasi di Linux (container cloud) pada HEAD:

| Pemeriksaan | Hasil |
|---|---|
| `cargo nextest run --workspace` | 1466/1466 lulus |
| `cargo test --doc` | 31 lulus |
| `cargo clippy --all-targets -D warnings` | bersih |
| Cross-clippy Windows (`--target x86_64-pc-windows-gnu`) | bersih |
| `cargo fmt --check` | bersih |
| Semua binary test sebagai user `nobody` (non-root, test izin akses ikut jalan) | lulus |

Catatan: run sebagai `nobody` dilakukan di `21c2694`. Setelah itu hanya ada dua fixup test-only untuk macOS (`broken_symlinks`, `security_context`).

CI GitHub:

- Run terakhir yang sudah selesai, #453 di `543fa2c`, merah. Penyebab yang **sudah diperbaiki** setelahnya:
  - Linux, Nix, coverage: `json_output_stress::test_json_permission_denied…` masih mengharapkan `"[]"` tanpa newline.
  - macOS: `broken_symlinks::empty_target` mengharapkan JSON tanpa newline.
  - macOS dan Windows: `security_context` memakai kolom `-Z`, yang hanya ada di Linux.
- Penyebab yang **belum diperbaiki**: lihat P1 di bawah (Windows `recurse_level`).
- CI untuk `ff3dc65` sudah di-dispatch, tapi hasilnya belum dicek: https://github.com/fxrdhan/lez/actions/workflows/ci.yml

## 3. Sisa pekerjaan (urut prioritas)

### P1: Windows CI masih merah: `recurse_level::recursion_stops_at_the_level_given`

Test ini mengoper path hasil `canonicalize`. Di Windows hasilnya berbentuk verbatim `\\?\C:\...`, dan `?` termasuk karakter shell-special, sehingga header rekursi jadi dikutip: `'\\?\C:\...\top\a':`.

Usulan perbaikan ada di `src/output/escape.rs`, fungsi `is_shell_special`. Di Windows, perlakukan `?` dan `*` sebagai karakter biasa, seperti `\` dan `[`. Alasannya: Windows melarang kedua karakter itu di nama file, jadi keduanya hanya muncul di prefix verbatim.

Langkahnya:

1. Ubah ekspresi menjadi `(cfg!(not(windows)) && matches!(c, '[' | '\\' | '?' | '*'))`, dan keluarkan `?`/`*` dari daftar umum.
2. Perbarui unit test `backslash_and_bracket_are_plain_on_windows`.
3. Perbarui catatan Windows di `man/lez.1.md` (`--quotes`).
4. Jadikan fixup ke commit `f408ca7 fix(quotes)`, lalu fixup man page ke `d63fd6c`.

Alternatif lain: test tidak memakai path `canonicalize`. Tapi user Windows juga bisa memberi path `\\?\`, jadi perbaikan di `escape.rs` lebih tepat.

### P1: Pastikan test baru hijau di macOS dan Windows

Test berikut ditulis atau diubah di sesi ini dan baru terbukti di Linux:

- `tests/os_metadata/*`: `permissions_special_bits` (setgid/sticky di macOS), `ls_colors_blocksize` (memakai `grouped()`), `xattr_display` (argumen file), `mount_indicators` (`--mounts`), `permissions_exit`
- `tests/icons_theme/special_dirs.rs`: ekspektasi macOS `Movies` = `\u{f03d}` setelah fix ikon
- `tests/icons_theme/dev_eclass_astro.rs`: `dev` dipindah ke `other/dev` agar aman di FS case-insensitive
- `tests/loc_engine/languages.rs`: tabel memakai `grouped()`. Format sudah dicocokkan dengan output macOS dari CI.
- `tests/cli_options/optional_values.rs`: `--absolute` memakai path apa adanya, tanpa canonicalize
- `tests/cli_options/config_file.rs`: path dibangun dengan `join`; kasus `security_context` hanya di Linux
- `tests/adversarial/json_output_stress.rs`: ditulis ulang; nama dengan karakter ilegal di Windows hanya dipakai di unix
- `tests/sorting/positional_args.rs`: JSON dibandingkan apa adanya

### P2: Verifikasi per commit (sempat dihentikan)

Setiap commit harus lulus clippy dan test sendiri (aturan AGENTS.md). Untuk `80f2c25..HEAD` belum dicek ulang setelah rebase. Contoh skrip:

```sh
for c in $(git rev-list --reverse b6dd399..HEAD); do
  git checkout -q --detach "$c"
  RUSTFLAGS="--deny warnings" cargo clippy --all-targets -q >/dev/null 2>&1 || { echo "CLIPPY-FAIL $c"; continue; }
  cargo nextest run --workspace --no-fail-fast >/dev/null 2>&1 || { echo "TEST-FAIL $c"; continue; }
  echo "OK $c $(git log -1 --format=%s)"
done
git switch c/practical-thompson-mhcnw3
```

Sebaiknya jalankan di worktree terpisah dengan `CARGO_TARGET_DIR` sendiri.

### P2: Verifikasi akhir

- Diff coverage: `cargo llvm-cov nextest` sekarang vs baseline `dev`. Baseline lama ada di container cloud dan hilang, jadi ukur ulang dari `dev`.
- `nix flake check -L`. Job Nix di CI akan menjalankannya juga.

### P3: Audit yang belum disentuh atau belum tuntas

Tujuan audit: tidak ada test yang "cheat" atau longgar. Bandingkan output utuh dengan oracle independen, lalu lakukan uji mutasi: kembalikan source ke versi lama dan pastikan test gagal.

Domain `adversarial`:

- `strict_mode_permutations.rs`: 36 `contains`, 10 `is_ok`/`is_err`. Paling longgar yang tersisa.
- `theme_yaml_fuzz_stress.rs`: 4 `contains`.
- `bug_remediations.rs:390`: `stderr.contains("Permission denied: ") && contains("code: 13")`. Ganti dengan pesan utuh, yaitu `"Permission denied: locked_dir - code: 13\n"`.
- `io_error_isolation.rs`: kasus JSON hanya mengecek beberapa field. Bandingkan dokumen utuh.
- `raw_bytes_paths.rs` (1 `contains`), `fd_exhaustion.rs` (1 `||`), `archive_fuzz_stress.rs` (1 `is_ok`).

Silent skip:

- `filesystem/symlink_broken_targets.rs:121` memakai `if geteuid() == 0 { return; }`. Ganti dengan `crate::common::permission_checks_apply()`, yang gagal keras di CI.

Lainnya perlu dinilai per kasus, karena sebagian `contains` sah (misalnya memeriksa bahwa dokumen menyebut sebuah flag):

- `cli_options/shell_completions.rs` (10 `contains`, 6 `||`)
- `man_pages.rs` (7 `contains`)
- `powertest_config.rs` (6 `contains`)
- `zsh_completions.rs` (3 `contains`)
- `sorting/recursive.rs` (3), `sorting/aliases.rs` (2)
- `platform/windows_paths.rs` (3 `contains`, 1 `||`)
- `cli_options/time_style_options.rs` (3 `is_ok`)

Belum dikerjakan sistematis: menambah test untuk kode yang belum ter-cover, berdasarkan laporan coverage.

## 4. Perubahan perilaku yang perlu direview sebelum merge

| Commit | Perubahan |
|---|---|
| `430b087` fix(tree) | `-T -f` sekarang menggambar cabang sesuai pohon utuh. Direktori yang disembunyikan tetap memberi garis batang. |
| `f408ca7` fix(quotes) | `--quotes=auto` kini mengutip semua karakter yang dikutip GNU `ls`: `` ! " $ & ' ( ) * ; < = > ? [ \ ^ ` | ``, ditambah `#`/`~` di awal nama. Apostrof memakai double quote hanya jika aman; selain itu `'\''`. Di Windows, `\` dan `[` dibiarkan polos. |
| `ae2ba56` fix(icons) | Ikon diwarnai sama dengan namanya, juga saat ada override tema. |
| `15d55b4` fix(json) | Dokumen `--json` diakhiri newline. |
| `5ed8049` fix(icons) | Folder khusus user (dari `user-dirs.dirs`/`$HOME`) mendapat ikonnya juga saat di-list lewat path relatif. Contoh di macOS: `Movies` konsisten memakai ikon video. |

## 5. Bug produk yang ditemukan dan diperbaiki di branch ini

Setiap perbaikan datang dengan test dan sudah diuji mutasi:

- fs: descriptor direktori bocor (`fb9cb71`); cache total-size memakai kunci yang salah (`df39e6e`)
- options: mock env membaca sistem asli (`52c7717`); nama variabel di pesan error (`7ba61cc`); pesan out-of-range untuk digit count dari env (`e00f338`)
- archives: ukuran entry dan record PAX (`7e0ea19`)
- json: code share (`bc90c3d`); newline di akhir dokumen (`15d55b4`)
- loc: bahasa link yang di-follow (`8e7edc0`); block comment bersarang (`738105c`)
- theme:
  - ikon bawaan hilang saat tema mengatur ikon default (`a10d5c7`)
  - `colourful` (`9c1f97d`)
  - base palette mengabaikan `--color-scale-mode` (`8f8072d`)
- tree: tepi `-T -f` (`430b087`)
- config:
  - parse error di file yang ditemukan otomatis kini dilaporkan (`23de2f3`)
  - key `absolute` diabaikan (`b107a02`)
- grid-details: spasi ganda (`cf6b4ff`); header kolom (`1781710`)
- code: escape tree saat warna mati (`d2cedee`)
- icons: warna ikon (`ae2ba56`); folder khusus dari path relatif (`5ed8049`)
- quotes: karakter shell-special (`f408ca7`)
- Docs: `cc0a7c6`, `80f2c25`, `be6bf94`, `003a06c`, `d63fd6c`

## 6. Observasi yang belum diperbaiki (kandidat issue)

- Di luar Linux, crate `locale` tidak membaca locale, sehingga angka selalu dikelompokkan gaya Inggris (`15,003`) walau `LANG=C`. Test memakai `common::grouped()`.
- Di macOS, `-Z` diterima tapi diam-diam tidak menampilkan apa-apa (kolom security context hanya ada di Linux).
- `ca` (capabilities) kalah dari warna ekstensi. Di GNU `ls`, `ca` menang.
- `--absolute=on` tidak me-resolve symlink di path (by design; `follow` yang me-resolve).
- Format JSON:
  - spasi tidak konsisten: `"Size": "1"` vs `"files":[...]`
  - tanggal JSON membawa padding tabel (`" 2 Oct 21:43"`)
  - `-R --json` dikunci per nama dir, sedangkan argumen ganda dikunci per path
  - `--no-symlink-targets` tetap menulis `Target` di JSON
  - `--code --json` mencetak tabel
- Grid tidak rata untuk emoji ZWJ (unicode-width lama lewat `ansi-width`).
- Marker truncation arsip "(0 B)".
- Hyperlink symlink menunjuk ke target.
- `l` mengambil warna `fi` saat `ln=target`.
- Docs config-dir macOS tidak cocok.
- `LEZ_CONFIG_FILE` kosong.
- plist membuang newline.
- Tally `--warn-hidden`.
- Batas `GIT_DIR` in-process.
- `TIME_STYLE` dan env luminance yang invalid diam-diam fallback.
- `-1 --tags` diam-diam mengabaikan tags.
- LOC heredoc (Ruby/Perl).
- Karakter kontrol tidak round-trip lewat shell.
- Odin tanpa ikon.
- Suite test tidak build dengan `--no-default-features`.
- Clippy `cmp_null` hanya di Windows.
- Nextest kadang melaporkan "leaky".

## 7. Diblok di cloud dan diserahkan ke kamu

- Menghapus branch-branch lama (stale).
- Rilis v0.28.5.
- Merge: kamu yang melakukannya.

## 8. Cara kerja yang dipakai (agar konsisten)

- Oracle independen, bukan menebak nilai:
  - output `ls`/`stat`/`getcap`/`getxattr`/`getpwuid_r`
  - flag CLI padanan untuk setiap key config
  - `std::io::Error` yang dibangun dari errno yang sama
  - `common::grouped()` untuk angka
- Uji mutasi untuk setiap perbaikan dan setiap test yang diketatkan.
- Perbaikan untuk commit lama: `git commit --fixup=<sha>` lalu `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash b6dd399`, cek bahwa tree tidak berubah, lalu `git push --force-with-lease`.
- Helper bersama ada di `tests/common/mod.rs`:
  - `lez_cmd`/`lez_in`, `success_stdout`, `TempTestDir`, `TempGitRepo`
  - `permission_checks_apply`, `set_xattr`, `grant_capabilities`
  - `grouped`, `require_git`
