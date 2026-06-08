//! Developer tasks for the `dioxus_codemirror` workspace.
//!
//! Currently a single task, `vendor`, which downloads CodeMirror and its
//! dependencies from esm.sh into `dioxus_codemirror/assets/codemirror-vendor/`
//! so the component has no runtime CDN dependency. Run with:
//!
//! ```sh
//! cargo run -p xtask -- vendor
//! ```
//!
//! Alongside the vendored modules it writes an `index.js` entry that re-exports
//! the core symbols plus a `languages` map of every vendored language. That
//! folder is served directly as the Dioxus asset (see `code_mirror.rs`): the
//! whole superset ships, because Dioxus cannot yet serve a per-feature folder
//! generated at build time -- <https://github.com/DioxusLabs/dioxus/issues/4426>.

use std::{
    collections::{BTreeSet, VecDeque},
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;

/// esm.sh origin.
const ESM: &str = "https://esm.sh";
/// Output directory for vendored modules, relative to the workspace root.
const OUT_DIR: &str = "dioxus_codemirror/assets/codemirror-vendor";

/// Packages the glue script always needs, regardless of language.
///
/// Their exported symbols are re-exported by the generated `index.js` (see
/// [`CodemirrorVendor::index_js_write`]). The crawler discovers transitive
/// dependencies automatically.
const CORE_ENTRIES: &[&str] = &[
    "codemirror",
    "@codemirror/state",
    "@codemirror/view",
    "@codemirror/language",
    "@codemirror/lsp-client",
    "@lezer/highlight",
];

/// One supported syntax-highlighting language.
///
/// `name` is the JS export symbol, the `Language` serde name, and the key in
/// the generated `languages` map -- all the same lowercase string, e.g.
/// `yaml`. Its Cargo feature is `lang-<name>`, e.g. `lang-yaml`.
struct LanguageEntry {
    /// esm.sh package, e.g. `@codemirror/lang-yaml`.
    package: &'static str,
    /// JS export / `Language` serde name / `languages` map key, e.g. `yaml`.
    name: &'static str,
}

/// The languages vendored into the superset.
///
/// Add an entry here (and a matching `Language` variant and `lang-*` Cargo
/// feature) to support another language; re-run `cargo run -p xtask -- vendor`.
const LANGUAGES: &[LanguageEntry] = &[
    LanguageEntry {
        package: "@codemirror/lang-yaml",
        name: "yaml",
    },
    LanguageEntry {
        package: "@codemirror/lang-markdown",
        name: "markdown",
    },
    LanguageEntry {
        package: "@codemirror/lang-javascript",
        name: "javascript",
    },
    LanguageEntry {
        package: "@codemirror/lang-css",
        name: "css",
    },
    LanguageEntry {
        package: "@codemirror/lang-html",
        name: "html",
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("vendor") => CodemirrorVendor::new().vendor_run(),
        other => {
            eprintln!("usage: cargo run -p xtask -- vendor");
            if let Some(other) = other {
                eprintln!("unknown task: {other:?}");
            }
            std::process::exit(2);
        }
    }
}

/// Downloads CodeMirror and its dependency closure from esm.sh, rewriting each
/// module's bare imports to sibling files so the vendored tree is self
/// contained.
struct CodemirrorVendor {
    /// Matches the specifier in `from "x"`, `import "x"`, and `import("x")`.
    import_regex: Regex,
}

/// Result of vendoring one module: its output stem and the modules it imports,
/// enqueued so the crawler reaches the whole dependency closure.
struct ModuleVendored {
    /// Modules to enqueue for crawling (bare names or absolute esm.sh paths).
    dep_modules: Vec<String>,
}

impl CodemirrorVendor {
    fn new() -> Self {
        let import_regex = Regex::new(r#"(?:from|import)\s*\(?\s*["']([^"']+)["']"#).unwrap();
        Self { import_regex }
    }

    fn vendor_run(&self) -> Result<(), Box<dyn Error>> {
        let out_dir = PathBuf::from(OUT_DIR);
        if out_dir.exists() {
            fs::remove_dir_all(&out_dir)?;
        }
        fs::create_dir_all(&out_dir)?;

        let mut queue: VecDeque<String> = CORE_ENTRIES
            .iter()
            .map(|entry| entry.to_string())
            .chain(LANGUAGES.iter().map(|lang| lang.package.to_string()))
            .collect();
        let mut done = BTreeSet::new();

        while let Some(module) = queue.pop_front() {
            if !done.insert(module.clone()) {
                continue;
            }
            let vendored = self.module_vendor(&module, &out_dir)?;
            for dep in vendored.dep_modules {
                if !done.contains(&dep) {
                    queue.push_back(dep);
                }
            }
        }

        Self::index_js_write(&out_dir)?;

        println!("\nvendored {} modules into {OUT_DIR}", done.len());
        Ok(())
    }

    /// Writes the `index.js` entry served as the Dioxus folder asset.
    ///
    /// It re-exports the fixed core symbols and imports every vendored language
    /// into a `languages` map keyed by name, which the glue script looks up.
    /// The whole superset is included regardless of the enabled `lang-*`
    /// features; see [`crate`] and <https://github.com/DioxusLabs/dioxus/issues/4426>.
    fn index_js_write(out_dir: &Path) -> Result<(), Box<dyn Error>> {
        let mut index_js = String::new();
        index_js.push_str("// Generated by `cargo run -p xtask -- vendor`. Do not edit.\n");
        index_js.push_str(
            "//\n\
             // Imports every vendored language. The whole superset ships because Dioxus\n\
             // cannot yet serve a build-script-generated, per-feature asset folder from\n\
             // `OUT_DIR` -- see https://github.com/DioxusLabs/dioxus/issues/4426. Once that\n\
             // lands, this entry can be trimmed to the enabled `lang-*` features again.\n",
        );
        index_js.push_str("export { EditorView, minimalSetup } from \"./codemirror.js\";\n");
        index_js.push_str(
            "export { EditorState, EditorSelection, Annotation } from \"./codemirror__state.js\";\n",
        );
        index_js.push_str(
            "export { lineNumbers, highlightActiveLineGutter, highlightActiveLine, \
             highlightWhitespace, rectangularSelection, crosshairCursor, keymap, \
             Decoration, ViewPlugin } \
             from \"./codemirror__view.js\";\n",
        );
        index_js.push_str(
            "export { HighlightStyle, syntaxHighlighting, bracketMatching, indentOnInput, \
             foldGutter, foldKeymap, codeFolding, foldable, foldEffect, unfoldAll, indentUnit } \
             from \"./codemirror__language.js\";\n",
        );
        index_js.push_str("export { SearchCursor } from \"./codemirror__search.js\";\n");
        index_js.push_str(
            "export { closeBrackets, closeBracketsKeymap } from \"./codemirror__autocomplete.js\";\n",
        );
        index_js.push_str("export { indentWithTab } from \"./codemirror__commands.js\";\n");
        index_js.push_str("export { tags } from \"./lezer__highlight.js\";\n");
        index_js.push_str(
            "export { LSPClient, LSPPlugin, languageServerExtensions } \
             from \"./codemirror__lsp-client.js\";\n",
        );

        for lang in LANGUAGES {
            index_js.push_str(&format!(
                "import {{ {export} }} from \"./{module}.js\";\n",
                export = lang.name,
                module = Self::file_stem(lang.package),
            ));
        }

        index_js.push_str("export const languages = {");
        for (index, lang) in LANGUAGES.iter().enumerate() {
            if index > 0 {
                index_js.push(',');
            }
            index_js.push_str(&format!(" {name}: {name}", name = lang.name));
        }
        index_js.push_str(" };\n");

        fs::write(out_dir.join("index.js"), index_js)?;
        println!("  index.js ({} languages)", LANGUAGES.len());
        Ok(())
    }

    /// Downloads one `module`, rewrites its imports to sibling files, writes it
    /// out, and returns the modules it imports.
    ///
    /// `module` is either a bare package name (e.g. `@codemirror/state`),
    /// fetched as esm.sh's `*` re-export stub, or an absolute esm.sh path (e.g.
    /// `/node/process.mjs` or `/*@lezer/lr@1.4.10/es2022/lr.mjs`) for the inner
    /// modules, polyfills, and chunks discovered while crawling. Stubs are
    /// vendored as-is (re-exporting their inner), so side-effect polyfill
    /// imports they carry are preserved like any other dependency.
    fn module_vendor(
        &self,
        module: &str,
        out_dir: &Path,
    ) -> Result<ModuleVendored, Box<dyn Error>> {
        // `source_path` is the module's own esm.sh path, used to resolve any
        // relative imports it contains. The `*` prefix marks dependencies
        // external, so esm.sh emits one file per package with bare imports.
        let (fetch_url, source_path, out_stem) = if module.starts_with('/') {
            (
                format!("{ESM}{module}"),
                module.to_string(),
                Self::path_stem(module),
            )
        } else {
            let spec = Self::package_spec(module);
            (
                format!("{ESM}/*{spec}?target=es2022"),
                String::new(),
                Self::file_stem(module),
            )
        };

        let mut code = Self::http_get(&fetch_url)?;

        let mut dep_modules = Vec::new();
        let specifiers: Vec<String> = self
            .import_regex
            .captures_iter(&code)
            .map(|capture| capture[1].to_string())
            .collect();
        for specifier in specifiers {
            // The regex also matches non-import uses of `from`/`import` in
            // minified code (e.g. `.from(",")`); ignore anything that is not a
            // plausible module specifier.
            if !Self::specifier_is_module(&specifier) {
                continue;
            }
            if specifier.contains("://") {
                return Err(
                    format!("{module}: unexpected absolute-URL import {specifier:?}").into(),
                );
            }
            // Resolve the import to a queue entry (bare name or absolute path)
            // and the sibling file it should point at.
            let (dep, dep_stem) = if specifier.starts_with('.') {
                let resolved = Self::path_resolve(&source_path, &specifier);
                let stem = Self::path_stem(&resolved);
                (resolved, stem)
            } else if specifier.starts_with('/') {
                (specifier.clone(), Self::path_stem(&specifier))
            } else {
                (specifier.clone(), Self::file_stem(&specifier))
            };

            if !dep_modules.contains(&dep) {
                dep_modules.push(dep);
            }
            let sibling = format!("./{dep_stem}.js");
            code = code
                .replace(&format!("\"{specifier}\""), &format!("\"{sibling}\""))
                .replace(&format!("'{specifier}'"), &format!("'{sibling}'"));
        }

        let file_path = out_dir.join(format!("{out_stem}.js"));
        fs::write(&file_path, code)?;
        println!("  {module} -> {}", file_path.display());
        Ok(ModuleVendored { dep_modules })
    }

    /// Whether `specifier` looks like a real module specifier rather than a
    /// regex false positive. Module specifiers start with `.`, `/`, `@`, or an
    /// alphanumeric, and contain only specifier characters.
    fn specifier_is_module(specifier: &str) -> bool {
        let Some(first) = specifier.chars().next() else {
            return false;
        };
        if !(first == '.' || first == '/' || first == '@' || first.is_ascii_alphanumeric()) {
            return false;
        }
        // `*` appears in esm.sh's external-build inner paths, e.g.
        // `/*@lezer/lr@1.4.10/es2022/lr.mjs`.
        specifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '/' | '@' | '-' | '_' | '*'))
    }

    /// Resolves a relative `specifier` against the directory of `source_path`,
    /// e.g. (`/a/b/c.mjs`, `../d.mjs`) -> `/a/d.mjs`.
    fn path_resolve(source_path: &str, specifier: &str) -> String {
        let mut segments: Vec<&str> = source_path
            .rsplit_once('/')
            .map(|(dir, _file)| dir)
            .unwrap_or("")
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        for segment in specifier.split('/') {
            match segment {
                "." | "" => {}
                ".." => {
                    segments.pop();
                }
                other => segments.push(other),
            }
        }
        format!("/{}", segments.join("/"))
    }

    /// File stem for an absolute esm.sh path, e.g. `/node/events.mjs` ->
    /// `node_events`. Non-alphanumeric characters become `_`.
    fn path_stem(path: &str) -> String {
        let trimmed = path.trim_start_matches('/').trim_end_matches(".mjs");
        trimmed
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// esm.sh fetch spec for a bare package name.
    ///
    /// Versions are pinned where a bare `@major` range resolves incorrectly --
    /// notably the meta `codemirror` package, whose `@6` resolves to an
    /// unrelated CodeMirror 5 lineage (`6.65.x`) on esm.sh.
    fn package_spec(name: &str) -> String {
        match name {
            "codemirror" => "codemirror@6.0.2".to_string(),
            "@codemirror/lang-yaml" => "@codemirror/lang-yaml@6.1.3".to_string(),
            "@codemirror/lang-markdown" => "@codemirror/lang-markdown@6.5.0".to_string(),
            "@codemirror/lsp-client" => "@codemirror/lsp-client@6.2.4".to_string(),
            _ if name.starts_with("@codemirror/") => format!("{name}@6"),
            _ if name.starts_with("@lezer/") => format!("{name}@1"),
            _ => name.to_string(),
        }
    }

    /// File stem for a package, e.g. `@codemirror/state` ->
    /// `codemirror__state`.
    fn file_stem(name: &str) -> String {
        name.trim_start_matches('@').replace('/', "__")
    }

    fn http_get(url: &str) -> Result<String, Box<dyn Error>> {
        Ok(ureq::get(url).call()?.into_string()?)
    }
}
