// Bridge between Dioxus (Rust/WASM) and CodeMirror 6.
//
// This script is run once per editor via `document::eval` and kept alive for
// the editor's lifetime, acting as a bidirectional message channel:
//
//   Rust -> JS   `await dioxus.recv()`  receives a `Cmd`:
//     { type: "init",  mount_id, doc, lsp_uri }   (always sent first)
//     { type: "doc_set", doc }
//     { type: "lsp_message_send", json }
//     { type: "destroy" }
//
//   JS -> Rust   `dioxus.send(..)`      sends an `Evt`:
//     { type: "ready" }
//     { type: "doc_changed", doc }
//     { type: "lsp_message_recv", json }
//
// CodeMirror is loaded at runtime from esm.sh, so there is no JS build step.
// To run fully offline, save the `?bundle` output of these URLs into an asset
// and import that local file instead.

const ESM = "https://esm.sh";
const CM = "6";

// Resolve the mount element, which may not be in the DOM yet on first render.
async function elementWait(id) {
  for (let attempt = 0; attempt < 600; attempt += 1) {
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

const [{ EditorView, basicSetup }, { EditorState, Annotation }, lspMod] =
  await Promise.all([
    import(`${ESM}/codemirror@${CM}?target=es2022`),
    import(`${ESM}/@codemirror/state@${CM}?target=es2022`),
    config.lsp_uri
      ? import(`${ESM}/@codemirror/lsp-client@${CM}?target=es2022`)
      : Promise.resolve(null),
  ]);

const parent = await elementWait(config.mount_id);

// Guard so programmatic `doc_set` updates do not echo back as `doc_changed`.
let applyingRemote = false;
const remoteAnnotation = Annotation.define();

const extensions = [
  basicSetup,
  EditorView.updateListener.of((update) => {
    if (update.docChanged && !applyingRemote) {
      dioxus.send({ type: "doc_changed", doc: update.state.doc.toString() });
    }
  }),
];

// === LSP wiring === //
// A message-based Transport that bridges the editor's LSP client to Rust:
// the client's outbound messages become `lsp_message_recv` events, and
// `lsp_message_send` commands are delivered to the client's subscribers.
let lspHandlers = [];
if (lspMod && config.lsp_uri) {
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

    const client = new lspMod.LSPClient({
      rootUri: config.lsp_uri.replace(/\/[^/]*$/, "") || config.lsp_uri,
      extensions: lspMod.languageServerExtensions(),
    }).connect(transport);

    extensions.push(client.plugin(config.lsp_uri));
  } catch (error) {
    console.warn("dioxus_codemirror: LSP client setup failed", error);
  }
}

const view = new EditorView({
  state: EditorState.create({ doc: config.doc ?? "", extensions }),
  parent,
});

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
