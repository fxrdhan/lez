// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::path::PathBuf;

use crate::fs::File;

impl File<'_> {
    /// For this file, return a vector of alternate file paths that, if any of
    /// them exist, mean that *this* file should be coloured as “compiled”.
    ///
    /// The point of this is to highlight compiled files such as `foo.js` when
    /// their source file `foo.coffee` exists in the same directory.
    /// For example, `foo.js` is perfectly valid without `foo.coffee`, so we
    /// don’t want to always blindly highlight `*.js` as compiled.
    /// (See also `FileType`)
    pub fn get_source_files(&self) -> Vec<PathBuf> {
        if let Some(ext) = &self.ext {
            match &ext[..] {
                "css"   => vec![self.path.with_extension("sass"), self.path.with_extension("scss"),  // SASS, SCSS
                                self.path.with_extension("styl"), self.path.with_extension("less")],  // Stylus, Less
                "mjs"   => vec![self.path.with_extension("mts")],  // JavaScript ES Modules source
                "cjs"   => vec![self.path.with_extension("cts")],  // JavaScript Commonjs Modules source
                "js"    => vec![self.path.with_extension("coffee"), self.path.with_extension("ts")],  // CoffeeScript, TypeScript
                "aux" |                                          // TeX: auxiliary file
                "bbl" |                                          // BibTeX bibliography file
                "bcf" |                                          // biblatex control file
                "blg" |                                          // BibTeX log file
                "fdb_latexmk" |                                  // TeX latexmk file
                "fls" |                                          // TeX -recorder file
                "headfootlength" |                               // TeX package autofancyhdr file
                "lof" |                                          // TeX list of figures
                "log" |                                          // TeX log file
                "lot" |                                          // TeX list of tables
                "out" |                                          // hyperref list of bookmarks
                "toc" |                                          // TeX table of contents
                "xdv" => vec![self.path.with_extension("tex")],  // XeTeX dvi

                _ => vec![],  // No source files if none of the above
            }
        } else {
            vec![] // No source files if there’s no extension, either!
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_source_files_mappings() {
        let make_file = |path_str: &str| {
            let p = Path::new(path_str);
            File::from_args(
                p.to_path_buf(),
                None,
                File::filename(p),
                false,
                false,
                false,
                None,
            )
        };

        // CSS sources
        let css_file = make_file("app.css");
        let css_sources = css_file.get_source_files();
        assert_eq!(
            css_sources,
            vec![
                PathBuf::from("app.sass"),
                PathBuf::from("app.scss"),
                PathBuf::from("app.styl"),
                PathBuf::from("app.less"),
            ]
        );

        // JS sources
        let js_file = make_file("bundle.js");
        let js_sources = js_file.get_source_files();
        assert_eq!(
            js_sources,
            vec![PathBuf::from("bundle.coffee"), PathBuf::from("bundle.ts")]
        );

        // MJS sources
        let mjs_file = make_file("server.mjs");
        assert_eq!(
            mjs_file.get_source_files(),
            vec![PathBuf::from("server.mts")]
        );

        // CJS sources
        let cjs_file = make_file("config.cjs");
        assert_eq!(
            cjs_file.get_source_files(),
            vec![PathBuf::from("config.cts")]
        );

        // TeX auxiliary sources
        let toc_file = make_file("paper.toc");
        assert_eq!(
            toc_file.get_source_files(),
            vec![PathBuf::from("paper.tex")]
        );

        // Unrelated extension
        let rs_file = make_file("main.rs");
        assert!(rs_file.get_source_files().is_empty());

        // No extension
        let no_ext_file = make_file("Makefile");
        assert!(no_ext_file.get_source_files().is_empty());
    }
}
