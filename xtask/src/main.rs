//! Developer tasks for the `dioxus_codemirror` workspace.
//!
//! Currently a single task, `vendor`, which downloads CodeMirror and its
//! dependencies from esm.sh into `dioxus_codemirror/assets/codemirror/` so the
//! component has no runtime CDN dependency. Run with:
//!
//! ```sh
//! cargo run -p xtask -- vendor
//! ```

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
const OUT_DIR: &str = "dioxus_codemirror/assets/codemirror";

/// Entry packages the glue script imports directly.
///
/// The crawler discovers their transitive dependencies automatically.
const ENTRIES: &[&str] = &[
    "codemirror",
    "@codemirror/state",
    "@codemirror/view",
    "@codemirror/lang-yaml",
    "@codemirror/lang-markdown",
    "@codemirror/lsp-client",
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

impl CodemirrorVendor {
    fn new() -> Self {
        let import_regex =
            Regex::new(r#"(?:from|import)\s*\(?\s*["']([^"']+)["']"#).unwrap();
        Self { import_regex }
    }

    fn vendor_run(&self) -> Result<(), Box<dyn Error>> {
        let out_dir = PathBuf::from(OUT_DIR);
        if out_dir.exists() {
            fs::remove_dir_all(&out_dir)?;
        }
        fs::create_dir_all(&out_dir)?;

        let mut queue: VecDeque<String> =
            ENTRIES.iter().map(|entry| entry.to_string()).collect();
        let mut done = BTreeSet::new();

        while let Some(module) = queue.pop_front() {
            if !done.insert(module.clone()) {
                continue;
            }
            for dep in self.module_vendor(&module, &out_dir)? {
                if !done.contains(&dep) {
                    queue.push_back(dep);
                }
            }
        }

        println!("\nvendored {} modules into {OUT_DIR}", done.len());
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
    ) -> Result<Vec<String>, Box<dyn Error>> {
        // `source_path` is the module's own esm.sh path, used to resolve any
        // relative imports it contains. The `*` prefix marks dependencies
        // external, so esm.sh emits one file per package with bare imports.
        let (fetch_url, source_path, out_stem) = if module.starts_with('/') {
            (format!("{ESM}{module}"), module.to_string(), Self::path_stem(module))
        } else {
            let spec = Self::package_spec(module);
            (format!("{ESM}/*{spec}?target=es2022"), String::new(), Self::file_stem(module))
        };

        let mut code = Self::http_get(&fetch_url)?;

        let mut deps = Vec::new();
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
                return Err(format!("{module}: unexpected absolute-URL import {specifier:?}").into());
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

            if !deps.contains(&dep) {
                deps.push(dep);
            }
            let sibling = format!("./{dep_stem}.js");
            code = code
                .replace(&format!("\"{specifier}\""), &format!("\"{sibling}\""))
                .replace(&format!("'{specifier}'"), &format!("'{sibling}'"));
        }

        let file_path = out_dir.join(format!("{out_stem}.js"));
        fs::write(&file_path, code)?;
        println!("  {module} -> {}", file_path.display());
        Ok(deps)
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
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
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

    /// File stem for a package, e.g. `@codemirror/state` -> `codemirror__state`.
    fn file_stem(name: &str) -> String {
        name.trim_start_matches('@').replace('/', "__")
    }

    fn http_get(url: &str) -> Result<String, Box<dyn Error>> {
        Ok(ureq::get(url).call()?.into_string()?)
    }
}
