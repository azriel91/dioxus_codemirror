// Bridge between Dioxus (Rust/WASM) and CodeMirror 6.
//
// This script is run once per editor via `document::eval` and kept alive for
// the editor's lifetime, acting as a bidirectional message channel:
//
//   Rust -> JS   `await dioxus.recv()`  receives a `Cmd`:
//     { type: "init", mount_id, doc, line_numbers, language, lsp_uri }  (first)
//     { type: "doc_set", doc }
//     { type: "lsp_message_send", json }
//     { type: "destroy" }
//
//   JS -> Rust   `dioxus.send(..)`      sends an `Evt`:
//     { type: "ready" }
//     { type: "doc_changed", doc }
//     { type: "lsp_message_recv", json }
//
// NOTE: `document::eval` runs this code via `new Function(..)`, where dynamic
// `import()` does not work. So CodeMirror is loaded once by an injected
// `<script type="module">` (which has proper module context) that imports the
// vendored modules and stashes them on `window.__dxcm`; this script just waits
// for them. The modules live in a Dioxus folder asset (`cm_base`) and are
// refreshed with `cargo run -p xtask -- vendor`.

// Module script that imports the vendored CodeMirror entry file (relative to
// `base`) and exposes its exports on `window.__dxcm`. Importing the entry pulls
// in its siblings, so the core `state`/`view` modules load exactly once and are
// shared (CodeMirror requires a single instance of each).
function codeMirrorLoaderScript(base) {
  // Import only the single entry. Dioxus bundles each vendored `.js` with
  // esbuild (inlining its imports), so importing several entry files would load
  // several copies of `@codemirror/state` and trip its "multiple instances"
  // check. One entry => one bundle => one shared `state` instance.
  //
  // The entry's exports vary with the consumer's enabled `lang-*` Cargo
  // features (see `build.rs`): the core symbols are always present, and a
  // `languages` map holds the bundled language factories keyed by name. Spread
  // the whole namespace so this script needs no per-language edits.
  const indexUrl = JSON.stringify(`${base}/index.js`);
  return `
(async () => {
  try {
    const cm = await import(${indexUrl});
    window.__dxcm = { ...cm };
  } catch (error) {
    window.__dxcmError = String(error);
    console.error("dioxus_codemirror: failed to load vendored CodeMirror", error);
  }
})();
`;
}

// Inject the loader once, then wait until the modules are available.
async function codeMirrorLoad(base) {
  if (!window.__dxcmInjected) {
    window.__dxcmInjected = true;
    const script = document.createElement("script");
    script.type = "module";
    script.textContent = codeMirrorLoaderScript(base);
    document.head.appendChild(script);
  }
  for (let attempt = 0; attempt < 1200; attempt += 1) {
    if (window.__dxcm) {
      return window.__dxcm;
    }
    if (window.__dxcmError) {
      throw new Error(`dioxus_codemirror: ${window.__dxcmError}`);
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
  throw new Error("dioxus_codemirror: timed out loading CodeMirror");
}

// Resolve the mount element, which may not be in the DOM yet on first render.
async function elementWait(id) {
  for (let attempt = 0; attempt < 1200; attempt += 1) {
    const element = document.getElementById(id);
    if (element) {
      return element;
    }
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
  throw new Error(`dioxus_codemirror: mount element #${id} not found`);
}

// The themeable CSS variables and their per-scheme source colors, each defined
// exactly once. `name` is the variable suffix, e.g. `syntax-keyword`; `light`
// and `dark` are that variable's source color in each scheme, e.g. `#cf222e` /
// `#ff7b72`. The stylesheet emits these as `--dxcm-light-<name>` /
// `--dxcm-dark-<name>` and then aliases the active `--dxcm-<name>` -- the only
// variables the chrome rules and the syntax `HighlightStyle` read -- to one
// scheme's sources or the other, so the colors are never duplicated.
const THEME_PALETTE = [
  { name: "bg", light: "#ffffff", dark: "#0d1117" },
  { name: "fg", light: "#1f2328", dark: "#e6edf3" },
  { name: "caret", light: "#1f2328", dark: "#e6edf3" },
  { name: "selection", light: "#d9d9d9", dark: "#2d333b" },
  { name: "selection-focused", light: "#c5dbff", dark: "#2f4b73" },
  { name: "selection-match", light: "#ffd33d55", dark: "#d2992255" },
  { name: "gutter-bg", light: "#f6f8fa", dark: "#0d1117" },
  { name: "gutter-fg", light: "#8c959f", dark: "#6e7681" },
  { name: "active-line", light: "#f0f3f6", dark: "#161b22" },
  { name: "active-line-gutter-bg", light: "#eaeef2", dark: "#161b22" },
  { name: "border", light: "#d0d7de", dark: "#30363d" },
  { name: "tooltip-bg", light: "#ffffff", dark: "#161b22" },
  { name: "tooltip-fg", light: "#1f2328", dark: "#e6edf3" },
  { name: "tooltip-selected-bg", light: "#0969da", dark: "#094771" },
  { name: "tooltip-selected-fg", light: "#ffffff", dark: "#ffffff" },
  { name: "tooltip-info-bg", light: "#f6f8fa", dark: "#0d1117" },
  { name: "syntax-keyword", light: "#cf222e", dark: "#ff7b72" },
  { name: "syntax-string", light: "#0a3069", dark: "#a5d6ff" },
  { name: "syntax-comment", light: "#6e7781", dark: "#8b949e" },
  { name: "syntax-number", light: "#0550ae", dark: "#79c0ff" },
  { name: "syntax-function", light: "#8250df", dark: "#d2a8ff" },
  { name: "syntax-type", light: "#953800", dark: "#ffa657" },
  { name: "syntax-constant", light: "#0550ae", dark: "#79c0ff" },
  { name: "syntax-operator", light: "#0550ae", dark: "#79c0ff" },
  { name: "syntax-property", light: "#116329", dark: "#7ee787" },
  { name: "syntax-heading", light: "#0550ae", dark: "#79c0ff" },
  { name: "syntax-link", light: "#0a3069", dark: "#a5d6ff" },
  { name: "syntax-invalid", light: "#cf222e", dark: "#ffa198" },
];

// `--dxcm-light-<name>: <light>; --dxcm-dark-<name>: <dark>;` for every entry --
// both palettes declared once.
const themePaletteSource = THEME_PALETTE.map(
  ({ name, light, dark }) =>
    `  --dxcm-light-${name}: ${light};\n  --dxcm-dark-${name}: ${dark};`,
).join("\n");

// `--dxcm-<name>: var(--dxcm-<scheme>-<name>);` for every entry -- points the
// active variables at the chosen scheme's sources, with no color duplicated.
function themeActivate(scheme) {
  return THEME_PALETTE.map(
    ({ name }) => `  --dxcm-${name}: var(--dxcm-${scheme}-${name});`,
  ).join("\n");
}

// Inject the editor chrome stylesheet once. Colors come from the active
// `--dxcm-<name>` variables, whose scheme follows the mount element's
// `data-theme` attribute (set from the `Theme` prop): `auto` (or absent) tracks
// the OS color scheme (`prefers-color-scheme`), while `light`/`dark` pin a
// palette regardless of the OS. So a single editor reads correctly in both
// modes with no consumer configuration, yet can be pinned per editor. Rules are
// scoped under `.dioxus-codemirror` (the mount div's class) so they win over
// CodeMirror's single-class base theme and never leak to the host page. Syntax
// token colors are applied separately, in JS, via a `HighlightStyle` (see
// `themeHighlightStyle`) because CodeMirror generates those token class names
// dynamically and they cannot be targeted from here.
function themeStylesInject() {
  if (window.__dxcmStyleInjected) {
    return;
  }
  window.__dxcmStyleInjected = true;
  const style = document.createElement("style");
  style.id = "dioxus-codemirror-theme";
  style.textContent = `
/* Both palette sources, plus the default (light) active aliases. */
.dioxus-codemirror {
${themePaletteSource}
${themeActivate("light")}
}

/* \`theme: Auto\` on a dark OS: re-alias the active variables to the dark
   sources. The \`:not\` leaves editors pinned to \`theme: Light\` untouched. */
@media (prefers-color-scheme: dark) {
  .dioxus-codemirror:not([data-theme="light"]) {
${themeActivate("dark")}
  }
}

/* \`theme: Dark\`: dark regardless of the OS. */
.dioxus-codemirror[data-theme="dark"] {
${themeActivate("dark")}
}

.dioxus-codemirror .cm-editor {
  background: var(--dxcm-bg);
  color: var(--dxcm-fg);
}
.dioxus-codemirror .cm-editor.cm-focused {
  outline: 1px solid var(--dxcm-border);
}
.dioxus-codemirror .cm-content {
  caret-color: var(--dxcm-caret);
}
.dioxus-codemirror .cm-cursor,
.dioxus-codemirror .cm-dropCursor {
  border-left-color: var(--dxcm-caret);
}
.dioxus-codemirror .cm-selectionBackground,
.dioxus-codemirror .cm-content ::selection {
  background: var(--dxcm-selection);
}
.dioxus-codemirror .cm-focused .cm-selectionBackground {
  background: var(--dxcm-selection-focused);
}
/* Other occurrences of the current selection (\`highlightSelectionMatches\`),
   themed so they track the color scheme. The match coinciding with the active
   selection (\`-main\`) is left transparent so the selection's own background
   (drawn by \`drawSelection\`) shows through instead of this highlight. */
.dioxus-codemirror .cm-selectionMatch {
  background: var(--dxcm-selection-match);
}
.dioxus-codemirror .cm-selectionMatch.cm-selectionMatch-main {
  background: transparent;
}
.dioxus-codemirror .cm-gutters {
  background: var(--dxcm-gutter-bg);
  color: var(--dxcm-gutter-fg);
  border-right-color: var(--dxcm-border);
}
.dioxus-codemirror .cm-activeLine {
  background: var(--dxcm-active-line);
}
.dioxus-codemirror .cm-activeLineGutter {
  background: var(--dxcm-active-line-gutter-bg);
  color: var(--dxcm-fg);
}

/* Autocomplete and hover tooltips. CodeMirror renders these inside the editor
   DOM (so they fall under \`.dioxus-codemirror\`) with only a single-class base
   theme, which these scoped rules outweigh. */
.dioxus-codemirror .cm-tooltip {
  background: var(--dxcm-tooltip-bg);
  color: var(--dxcm-tooltip-fg);
  border: 1px solid var(--dxcm-border);
  border-radius: 6px;
}
.dioxus-codemirror .cm-tooltip-autocomplete ul li {
  color: var(--dxcm-tooltip-fg);
}
.dioxus-codemirror .cm-tooltip-autocomplete ul li[aria-selected] {
  background: var(--dxcm-tooltip-selected-bg);
  color: var(--dxcm-tooltip-selected-fg);
}
.dioxus-codemirror .cm-completionInfo {
  background: var(--dxcm-tooltip-info-bg);
  color: var(--dxcm-tooltip-fg);
  border: 1px solid var(--dxcm-border);
}
`;
  document.head.appendChild(style);
}

// Syntax highlighting whose colors are the `--dxcm-syntax-*` CSS variables, so
// token colors track the OS color scheme alongside the chrome (see
// `themeStylesInject`). Layered after `minimalSetup`'s fallback default style,
// which then only applies to tags this style leaves unstyled.
function themeHighlightStyle() {
  return HighlightStyle.define([
    { tag: tags.keyword, color: "var(--dxcm-syntax-keyword)" },
    {
      tag: [tags.name, tags.deleted, tags.character, tags.macroName],
      color: "var(--dxcm-fg)",
    },
    {
      tag: [tags.propertyName, tags.attributeName],
      color: "var(--dxcm-syntax-property)",
    },
    {
      tag: [tags.function(tags.variableName), tags.labelName],
      color: "var(--dxcm-syntax-function)",
    },
    {
      tag: [tags.color, tags.constant(tags.name), tags.standard(tags.name)],
      color: "var(--dxcm-syntax-constant)",
    },
    {
      tag: [tags.typeName, tags.className, tags.namespace, tags.changed, tags.annotation, tags.modifier, tags.self],
      color: "var(--dxcm-syntax-type)",
    },
    {
      tag: [tags.number, tags.integer, tags.float, tags.atom, tags.bool],
      color: "var(--dxcm-syntax-number)",
    },
    {
      tag: [tags.operator, tags.operatorKeyword, tags.escape, tags.regexp, tags.special(tags.string)],
      color: "var(--dxcm-syntax-operator)",
    },
    {
      tag: [tags.meta, tags.comment],
      color: "var(--dxcm-syntax-comment)",
      fontStyle: "italic",
    },
    {
      tag: [tags.string, tags.inserted, tags.processingInstruction],
      color: "var(--dxcm-syntax-string)",
    },
    {
      tag: [tags.url, tags.link],
      color: "var(--dxcm-syntax-link)",
      textDecoration: "underline",
    },
    { tag: tags.heading, color: "var(--dxcm-syntax-heading)", fontWeight: "bold" },
    { tag: tags.strong, fontWeight: "bold" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strikethrough, textDecoration: "line-through" },
    { tag: tags.invalid, color: "var(--dxcm-syntax-invalid)" },
  ]);
}

// `Mod-d` command: add the next occurrence of the current selection as an extra
// selection range. Unlike CodeMirror's `selectNextOccurrence`, this matches
// substrings -- it does not restrict a whole-word selection to whole-word
// matches -- so selecting "hello" also extends into "helloabcd". Requires
// `allow_multiple_selections`.
function selectNextMatch(view) {
  const { state } = view;
  const { selection } = state;
  const main = selection.main;

  // First press on a bare cursor: select the word under it (the term to extend).
  if (main.empty) {
    const word = state.wordAt(main.head);
    if (!word) {
      return false;
    }
    view.dispatch({
      selection: EditorSelection.create(
        selection.ranges.map((range, index) =>
          index === selection.mainIndex
            ? EditorSelection.range(word.from, word.to)
            : range,
        ),
        selection.mainIndex,
      ),
    });
    return true;
  }

  // Every existing range must hold the same text, else there is no single term
  // to extend (mirrors CodeMirror's behaviour).
  const query = state.sliceDoc(main.from, main.to);
  const sameText = selection.ranges.every(
    (range) => state.sliceDoc(range.from, range.to) === query,
  );
  if (!query || !sameText) {
    return false;
  }

  const next = selectNextMatchFind(state, query, selection.ranges);
  if (!next) {
    return false;
  }

  view.dispatch({
    selection: selection.addRange(EditorSelection.range(next.from, next.to)),
    scrollIntoView: true,
  });
  return true;
}

// Next occurrence of `query` after the last selection range, wrapping around to
// the start, skipping ranges that are already selected.
function selectNextMatchFind(state, query, ranges) {
  const taken = new Set(ranges.map((range) => `${range.from}:${range.to}`));
  const after = ranges[ranges.length - 1].to;
  const scan = (from, to) => {
    const cursor = new SearchCursor(state.doc, query, from, to);
    while (!cursor.next().done) {
      if (!taken.has(`${cursor.value.from}:${cursor.value.to}`)) {
        return cursor.value;
      }
    }
    return null;
  };
  return scan(after, state.doc.length) ?? scan(0, after);
}

// The first message from Rust is always the init config.
const config = await dioxus.recv();

const {
  EditorView,
  minimalSetup,
  EditorState,
  EditorSelection,
  Annotation,
  lineNumbers,
  highlightActiveLineGutter,
  highlightActiveLine,
  highlightWhitespace,
  rectangularSelection,
  crosshairCursor,
  keymap,
  HighlightStyle,
  syntaxHighlighting,
  bracketMatching,
  indentOnInput,
  highlightSelectionMatches,
  SearchCursor,
  closeBrackets,
  closeBracketsKeymap,
  tags,
  LSPClient,
  languageServerExtensions,
  // Map of bundled language factories keyed by name, e.g. `{ yaml, markdown }`.
  // Which languages are present depends on the enabled `lang-*` Cargo features.
  languages,
} = await codeMirrorLoad(config.cm_base);

// Guard so programmatic `doc_set` updates do not echo back as `doc_changed`.
let applyingRemote = false;
const remoteAnnotation = Annotation.define();

// Inject the chrome stylesheet before the editor mounts so its first paint is
// already themed.
themeStylesInject();

// `minimalSetup` keeps the editor editable (history, default keymap, syntax
// highlighting) without imposing a line-number gutter. `themeHighlightStyle`
// follows it so its token colors take precedence over the fallback default,
// and both track the OS color scheme via the injected CSS variables.
const extensions = [
  minimalSetup,
  syntaxHighlighting(themeHighlightStyle()),
  EditorView.updateListener.of((update) => {
    if (update.docChanged && !applyingRemote) {
      dioxus.send({ type: "doc_changed", doc: update.state.doc.toString() });
    }
  }),
];

if (config.line_numbers) {
  extensions.push(lineNumbers(), highlightActiveLineGutter());
}

// === Optional editor features === //
// Each maps to the CodeMirror extension of the same name, toggled by a flag in
// the init config (see the matching `CodeMirror` props). `minimalSetup` already
// includes `drawSelection` and the default keymap, so multiple selections only
// need the facet and added keymaps layer on top of the defaults.
if (config.allow_multiple_selections) {
  extensions.push(EditorState.allowMultipleSelections.of(true));
}
if (config.highlight_active_line) {
  extensions.push(highlightActiveLine());
}
if (config.highlight_selection_matches) {
  // Highlight other occurrences of the current word/selection, and bind `Mod-d`
  // to extend the selection to the next occurrence. `selectNextMatch` matches
  // substrings, unlike the search keymap's `selectNextOccurrence`.
  extensions.push(
    highlightSelectionMatches(),
    keymap.of([{ key: "Mod-d", run: selectNextMatch, preventDefault: true }]),
  );
}
if (config.bracket_matching) {
  extensions.push(bracketMatching());
}
if (config.close_brackets) {
  extensions.push(closeBrackets(), keymap.of(closeBracketsKeymap));
}
if (config.rectangular_selection) {
  extensions.push(rectangularSelection(), crosshairCursor());
}
if (config.indent_on_input) {
  extensions.push(indentOnInput());
}
if (config.highlight_whitespace) {
  extensions.push(highlightWhitespace());
}
if (config.line_wrapping) {
  extensions.push(EditorView.lineWrapping);
}
if (config.read_only) {
  extensions.push(EditorState.readOnly.of(true));
}
if (typeof config.tab_size === "number") {
  extensions.push(EditorState.tabSize.of(config.tab_size));
}

// Apply the syntax extension for the requested language, if it was bundled. A
// language is bundled only when its `lang-*` Cargo feature is enabled; an
// un-bundled language falls back to plain text rather than failing.
if (config.language) {
  const languageFactory = languages?.[config.language];
  if (languageFactory) {
    extensions.push(languageFactory());
  } else {
    console.warn(
      `dioxus_codemirror: language "${config.language}" is not bundled; ` +
        `enable its Cargo feature (lang-${config.language}) on dioxus_codemirror`,
    );
  }
}

// === LSP wiring === //
// A message-based Transport that bridges the editor's LSP client to Rust:
// the client's outbound messages become `lsp_message_recv` events, and
// `lsp_message_send` commands are delivered to the client's subscribers.
let lspHandlers = [];
if (config.lsp_uri) {
  try {
    const transport = {
      send(message) {
        dioxus.send({ type: "lsp_message_recv", json: message });
      },
      subscribe(handler) {
        lspHandlers.push(handler);
      },
      unsubscribe(handler) {
        lspHandlers = lspHandlers.filter((h) => h !== handler);
      },
    };

    const client = new LSPClient({
      rootUri: config.lsp_uri.replace(/\/[^/]*$/, "") || config.lsp_uri,
      // Generous timeout: the request/response round trip crosses the Rust
      // (WASM) boundary and is driven by the Dioxus runtime, which can be slow
      // during initial page load. The default is 3s.
      timeout: 30000,
      extensions: languageServerExtensions(),
    }).connect(transport);

    extensions.push(client.plugin(config.lsp_uri));
  } catch (error) {
    console.warn("dioxus_codemirror: LSP client setup failed", error);
  }
}

const parent = await elementWait(config.mount_id);
let view;
try {
  view = new EditorView({
    state: EditorState.create({ doc: config.doc ?? "", extensions }),
    parent,
  });
} catch (error) {
  console.error("dioxus_codemirror: editor creation failed for", config.mount_id, error);
  throw error;
}

dioxus.send({ type: "ready" });

// === Command loop === //
while (true) {
  let cmd;
  try {
    cmd = await dioxus.recv();
  } catch (error) {
    // Channel closed -- the component unmounted.
    break;
  }

  switch (cmd.type) {
    case "doc_set": {
      const current = view.state.doc.toString();
      if (current === cmd.doc) {
        break;
      }
      applyingRemote = true;
      view.dispatch({
        changes: { from: 0, to: current.length, insert: cmd.doc },
        annotations: remoteAnnotation.of(true),
      });
      applyingRemote = false;
      break;
    }
    case "lsp_message_send": {
      for (const handler of lspHandlers) {
        handler(cmd.json);
      }
      break;
    }
    case "destroy": {
      view.destroy();
      return;
    }
    default:
      console.warn("dioxus_codemirror: unknown command", cmd);
  }
}
