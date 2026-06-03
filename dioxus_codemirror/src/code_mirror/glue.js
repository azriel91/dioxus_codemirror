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

// The first message from Rust is always the init config.
const config = await dioxus.recv();

const {
  EditorView,
  minimalSetup,
  EditorState,
  Annotation,
  lineNumbers,
  highlightActiveLineGutter,
  yaml,
  markdown,
  LSPClient,
  languageServerExtensions,
} = await codeMirrorLoad(config.cm_base);

// Guard so programmatic `doc_set` updates do not echo back as `doc_changed`.
let applyingRemote = false;
const remoteAnnotation = Annotation.define();

// `minimalSetup` keeps the editor editable (history, default keymap, syntax
// highlighting) without imposing a line-number gutter.
const extensions = [
  minimalSetup,
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
