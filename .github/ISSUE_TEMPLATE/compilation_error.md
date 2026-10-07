---
name: Compilation error
about: Report a problem compiling lez
title: 'build: '
labels: 'bug'
assignees: ''

---

If lez fails to compile, or if there is a problem during the build process, then please include the following information in your report:

- The exact lez commit you are building (`git rev-parse --short HEAD`), or the version you are installing from crates.io
- The version of rustc you are compiling it with (`rustc --version`); lez needs Rust 1.90 or later
- The Cargo features you are building with, if not the default ones (for example `--no-default-features`)
- Your operating system and hardware platform
- The Rust build target (the _exact_ output of `rustc --print cfg`)

If you are seeing compilation errors, please include the output of the build process.

---
