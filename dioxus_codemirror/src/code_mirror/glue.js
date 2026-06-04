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

// Module script that imports the vendored CodeMirror entry files (relative to
// `base`) and exposes them on `window.__dxcm`. Importing the entries pulls in
// their siblings, so the core `state`/`view` modules load exactly once and are
// shared (CodeMirror requires a single instance of each).
function codeMirrorLoaderScript(base) {
  // Import only the single entry. Dioxus bundles each vendored `.js` with
  // esbuild (inlining its imports), so importing several entry files would load
  // several copies of `@codemirror/state` and trip its "multiple instances"
  // check. One entry => one bundle => one shared `state` instance.
  const indexUrl = JSON.stringify(`${base}/index.js`);
  return `
(async () => {
  try {
    const cm = await import(${indexUrl});
    window.__dxcm = {
      EditorView: cm.EditorView,
      minimalSetup: cm.minimalSetup,
      EditorState: cm.EditorState,
      Annotation: cm.Annotation,
      lineNumbers: cm.lineNumbers,
      highlightActiveLineGutter: cm.highlightActiveLineGutter,
      HighlightStyle: cm.HighlightStyle,
      syntaxHighlighting: cm.syntaxHighlighting,
      tags: cm.tags,
      yaml: cm.yaml,
      markdown: cm.markdown,
      LSPClient: cm.LSPClient,
      languageServerExtensions: cm.languageServerExtensions,
    };
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

// Inject the editor chrome stylesheet once. The colors are CSS variables whose
// palette is chosen by the mount element's `data-theme` attribute (set from the
// `Theme` prop): `auto` (or absent) follows the OS color scheme
// (`prefers-color-scheme`), while `light`/`dark` force a palette regardless of
// the OS. So a single editor reads correctly in both modes with no consumer
// configuration, yet can be pinned per editor. Rules are scoped under
// `.dioxus-codemirror` (the mount div's class) so they win over CodeMirror's
// single-class base theme and never leak to the host page. Syntax token colors
// are applied separately, in JS, via a `HighlightStyle` (see
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
/* Light palette: the default, and also \`theme: Light\` forced on a dark OS
   (the attribute selector outranks the bare-class dark rules below). */
.dioxus-codemirror,
.dioxus-codemirror[data-theme="light"] {
  --dxcm-bg: #ffffff;
  --dxcm-fg: #1f2328;
  --dxcm-caret: #1f2328;
  --dxcm-selection: #d9d9d9;
  --dxcm-selection-focused: #c5dbff;
  --dxcm-gutter-bg: #f6f8fa;
  --dxcm-gutter-fg: #8c959f;
  --dxcm-active-line: #f0f3f6;
  --dxcm-active-line-gutter-bg: #eaeef2;
  --dxcm-border: #d0d7de;
  --dxcm-syntax-keyword: #cf222e;
  --dxcm-syntax-string: #0a3069;
  --dxcm-syntax-comment: #6e7781;
  --dxcm-syntax-number: #0550ae;
  --dxcm-syntax-function: #8250df;
  --dxcm-syntax-type: #953800;
  --dxcm-syntax-constant: #0550ae;
  --dxcm-syntax-operator: #0550ae;
  --dxcm-syntax-property: #116329;
  --dxcm-syntax-heading: #0550ae;
  --dxcm-syntax-link: #0a3069;
  --dxcm-syntax-invalid: #cf222e;
}

/* Dark palette, written once as a reusable list and applied to the two cases
   that need it: \`theme: Auto\` on a dark OS, and \`theme: Dark\` always. */
@media (prefers-color-scheme: dark) {
  .dioxus-codemirror:not([data-theme="light"]) {
    --dxcm-bg: #0d1117;
    --dxcm-fg: #e6edf3;
    --dxcm-caret: #e6edf3;
    --dxcm-selection: #2d333b;
    --dxcm-selection-focused: #2f4b73;
    --dxcm-gutter-bg: #0d1117;
    --dxcm-gutter-fg: #6e7681;
    --dxcm-active-line: #161b22;
    --dxcm-active-line-gutter-bg: #161b22;
    --dxcm-border: #30363d;
    --dxcm-syntax-keyword: #ff7b72;
    --dxcm-syntax-string: #a5d6ff;
    --dxcm-syntax-comment: #8b949e;
    --dxcm-syntax-number: #79c0ff;
    --dxcm-syntax-function: #d2a8ff;
    --dxcm-syntax-type: #ffa657;
    --dxcm-syntax-constant: #79c0ff;
    --dxcm-syntax-operator: #79c0ff;
    --dxcm-syntax-property: #7ee787;
    --dxcm-syntax-heading: #79c0ff;
    --dxcm-syntax-link: #a5d6ff;
    --dxcm-syntax-invalid: #ffa198;
  }
}

.dioxus-codemirror[data-theme="dark"] {
  --dxcm-bg: #0d1117;
  --dxcm-fg: #e6edf3;
  --dxcm-caret: #e6edf3;
  --dxcm-selection: #2d333b;
  --dxcm-selection-focused: #2f4b73;
  --dxcm-gutter-bg: #0d1117;
  --dxcm-gutter-fg: #6e7681;
  --dxcm-active-line: #161b22;
  --dxcm-active-line-gutter-bg: #161b22;
  --dxcm-border: #30363d;
  --dxcm-syntax-keyword: #ff7b72;
  --dxcm-syntax-string: #a5d6ff;
  --dxcm-syntax-comment: #8b949e;
  --dxcm-syntax-number: #79c0ff;
  --dxcm-syntax-function: #d2a8ff;
  --dxcm-syntax-type: #ffa657;
  --dxcm-syntax-constant: #79c0ff;
  --dxcm-syntax-operator: #79c0ff;
  --dxcm-syntax-property: #7ee787;
  --dxcm-syntax-heading: #79c0ff;
  --dxcm-syntax-link: #a5d6ff;
  --dxcm-syntax-invalid: #ffa198;
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

// The first message from Rust is always the init config.
const config = await dioxus.recv();

const {
  EditorView,
  minimalSetup,
  EditorState,
  Annotation,
  lineNumbers,
  highlightActiveLineGutter,
  HighlightStyle,
  syntaxHighlighting,
  tags,
  yaml,
  markdown,
  LSPClient,
  languageServerExtensions,
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

if (config.language === "yaml") {
  extensions.push(yaml());
} else if (config.language === "markdown") {
  extensions.push(markdown());
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
