//! Generates the served CodeMirror asset folder from the enabled `lang-*`
//! Cargo features.
//!
//! `cargo run -p xtask -- vendor` downloads the full language superset into
//! `assets/codemirror-vendor/` and records a `manifest.json` of per-language
//! file closures. This script reads that manifest, then copies only the files
//! needed by the enabled languages into `assets/codemirror/` (the folder
//! `code_mirror.rs` serves via `asset!`) and writes a matching `index.js` entry
//! that re-exports the core symbols plus a `languages` map of the enabled
//! languages. So a consumer ships exactly the languages they opt into, and
//! nothing more.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

/// Vendored superset, read-only input. Committed to the repository.
const VENDOR_DIR: &str = "assets/codemirror-vendor";
/// Served asset folder, generated output. Git-ignored.
const GEN_DIR: &str = "assets/codemirror";
/// Records the signature of the last generated folder, so an unchanged build
/// skips rewriting (avoiding asset re-hashing). Sits beside the served folder,
/// not inside it, so it is never itself served. Git-ignored.
const MARKER: &str = "assets/.codemirror-features";

/// Manifest written by `xtask`, describing the vendored superset.
#[derive(Deserialize)]
struct Manifest {
    /// File stems always copied, regardless of language.
    core: Vec<String>,
    /// Per-language file closures.
    languages: Vec<LanguageManifest>,
}

/// Manifest entry for a single language.
#[derive(Deserialize)]
struct LanguageManifest {
    /// Export name / `languages` map key, e.g. `yaml`.
    name: String,
    /// Cargo feature that enables it, e.g. `lang-yaml`.
    feature: String,
    /// File stem of the entry module to import, e.g. `codemirror__lang-yaml`.
    module: String,
    /// JS export symbol to import from the entry module, e.g. `yaml`.
    export: String,
    /// File stems this language needs (its full closure).
    files: Vec<String>,
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"));
    let vendor_dir = manifest_dir.join(VENDOR_DIR);
    let gen_dir = manifest_dir.join(GEN_DIR);
    let marker_path = manifest_dir.join(MARKER);
    let manifest_path = vendor_dir.join("manifest.json");

    // Force this script to run on every build. The served folder is shared
    // global state (it must live at a fixed path for `asset!`, so it cannot go
    // in `OUT_DIR`); a build for a different `lang-*` feature set could leave it
    // stale when cargo would otherwise reuse this feature set's cached build. We
    // re-run every time and reconcile, but only rewrite when the output content
    // actually changes (see the marker check), so unchanged builds are cheap and
    // do not churn asset hashes. Re-running is forced by depending on a stamp
    // file we rewrite with a fresh value each run.
    let stamp_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("dxcm-rerun-stamp");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    fs::write(&stamp_path, stamp.to_string()).expect("write rerun stamp");
    println!("cargo:rerun-if-changed={}", stamp_path.display());
    println!("cargo:rerun-if-changed={}", vendor_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = manifest_read(&manifest_path);

    let languages_enabled: Vec<&LanguageManifest> = manifest
        .languages
        .iter()
        .filter(|language| feature_is_enabled(&language.feature))
        .collect();

    // Files to serve = core closure union the enabled languages' closures.
    let mut file_stems: BTreeSet<&str> = manifest.core.iter().map(String::as_str).collect();
    for language in &languages_enabled {
        for file_stem in &language.files {
            file_stems.insert(file_stem.as_str());
        }
    }

    let index_js = index_js_render(&languages_enabled);

    // Skip rewriting when nothing affecting the output changed. The signature
    // captures both the file set and the generated entry, so a feature change,
    // a re-vendor that alters a closure, or an `index.js` format change all
    // trigger regeneration, while a no-op rebuild leaves the folder untouched.
    let signature = signature_of(&file_stems, &index_js);
    let signature_unchanged = fs::read_to_string(&marker_path)
        .is_ok_and(|previous| previous == signature)
        && gen_dir.join("index.js").exists();
    if signature_unchanged {
        return;
    }

    gen_dir_write(&vendor_dir, &gen_dir, &file_stems, &index_js);
    fs::write(&marker_path, &signature).expect("write marker");
}

/// A content signature of the folder to generate: the sorted file set followed
/// by the generated `index.js`.
fn signature_of(file_stems: &BTreeSet<&str>, index_js: &str) -> String {
    let mut signature = String::new();
    for file_stem in file_stems {
        signature.push_str(file_stem);
        signature.push('\n');
    }
    signature.push_str("--index.js--\n");
    signature.push_str(index_js);
    signature
}

/// Reads and parses the manifest, with a hint to re-vendor if it is missing.
fn manifest_read(manifest_path: &Path) -> Manifest {
    let manifest_json = fs::read_to_string(manifest_path).unwrap_or_else(|error| {
        panic!(
            "dioxus_codemirror build.rs: failed to read {}: {error}.\n\
             Run `cargo run -p xtask -- vendor` to (re)generate the vendored assets.",
            manifest_path.display()
        )
    });
    serde_json::from_str(&manifest_json).unwrap_or_else(|error| {
        panic!(
            "dioxus_codemirror build.rs: failed to parse {}: {error}",
            manifest_path.display()
        )
    })
}

/// Whether the Cargo `feature` (e.g. `lang-yaml`) is enabled, via the
/// `CARGO_FEATURE_LANG_YAML` environment variable cargo sets per build.
fn feature_is_enabled(feature: &str) -> bool {
    let var = format!("CARGO_FEATURE_{}", feature.to_uppercase().replace('-', "_"));
    env::var_os(var).is_some()
}

/// Recreates `gen_dir`, copies the selected files from `vendor_dir`, and writes
/// the generated `index.js`.
fn gen_dir_write(vendor_dir: &Path, gen_dir: &Path, file_stems: &BTreeSet<&str>, index_js: &str) {
    if gen_dir.exists() {
        fs::remove_dir_all(gen_dir)
            .unwrap_or_else(|error| panic!("remove {}: {error}", gen_dir.display()));
    }
    fs::create_dir_all(gen_dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", gen_dir.display()));

    for file_stem in file_stems {
        let file_name = format!("{file_stem}.js");
        let src = vendor_dir.join(&file_name);
        let dst = gen_dir.join(&file_name);
        fs::copy(&src, &dst)
            .unwrap_or_else(|error| panic!("copy {} -> {}: {error}", src.display(), dst.display()));
    }

    fs::write(gen_dir.join("index.js"), index_js)
        .unwrap_or_else(|error| panic!("write index.js: {error}"));
}

/// Renders the `index.js` entry: fixed core re-exports plus a `languages` map
/// of the enabled languages, which the glue script looks up by name.
fn index_js_render(languages_enabled: &[&LanguageManifest]) -> String {
    let mut index_js = String::new();
    index_js.push_str(
        "// Generated by dioxus_codemirror's build.rs from the enabled `lang-*` Cargo\n\
         // features. Do not edit; change the features in Cargo.toml instead.\n",
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
        "export { HighlightStyle, syntaxHighlighting, bracketMatching, indentOnInput } \
         from \"./codemirror__language.js\";\n",
    );
    index_js.push_str("export { SearchCursor } from \"./codemirror__search.js\";\n");
    index_js.push_str(
        "export { closeBrackets, closeBracketsKeymap } from \"./codemirror__autocomplete.js\";\n",
    );
    index_js.push_str("export { indentWithTab } from \"./codemirror__commands.js\";\n");
    index_js.push_str("export { tags } from \"./lezer__highlight.js\";\n");
    index_js.push_str(
        "export { LSPClient, languageServerExtensions } from \"./codemirror__lsp-client.js\";\n",
    );

    for language in languages_enabled {
        index_js.push_str(&format!(
            "import {{ {export} }} from \"./{module}.js\";\n",
            export = language.export,
            module = language.module,
        ));
    }

    index_js.push_str("export const languages = {");
    for (index, language) in languages_enabled.iter().enumerate() {
        if index > 0 {
            index_js.push(',');
        }
        index_js.push_str(&format!(
            " {name}: {export}",
            name = language.name,
            export = language.export,
        ));
    }
    index_js.push_str(" };\n");

    index_js
}
