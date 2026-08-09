# OpenPencil for VS Code

Design-as-code inside your editor — open, view, and edit OpenPencil `.op` files, wire your AI tools to the OpenPencil MCP server, and generate framework code from a design.

## Features

- **Custom `.op` editor** — `.op` design files open in a live OpenPencil canvas hosted in a VS Code webview, backed by a local OpenPencil daemon. Edits are tracked as document changes, so save, save-as, revert, and hot-exit backup all work like a native editor.
- **`.fig` import** — opening a `.fig` file converts it into design content on the fly; saves land on the sibling `.op` file (for example `design.fig` → `design.op`) rather than writing back to the `.fig`.
- **MCP configuration** — the extension runs a local MCP proxy and can write the connection into your IDE's MCP config, so an AI assistant can drive OpenPencil through the Model Context Protocol. Commands: *Configure MCP* and *Remove MCP*.
- **AI skill install** — install (or remove) the bundled OpenPencil design skill into your IDE's rules directory (Cursor / Trae / Windsurf); VS Code itself drives OpenPencil through MCP and needs no rules file. Commands: *Install Skill* and *Remove Skill*.
- **Code generation** — generate React or Vue code from the active design. When VS Code's language-model API is available it runs directly; otherwise it hands off a pre-filled prompt to your IDE's chat. Command: *Generate Code*. The target framework is configurable (`react` or `vue`).
- **`@openpencil` chat participant** — an OpenPencil design assistant registered in VS Code chat.
- **Workspace-trust aware** — in an untrusted workspace `.op` files show a read-only placeholder and no local daemon or MCP config is started; granting trust assembles the full stack and reopens the files.

## Requirements

- **VS Code** `^1.90.0` or newer.
- An **OpenPencil `op-host-web-server` daemon binary**. The extension probes `<workspace>/target/debug/op-host-web-server` by default; set `openpencil.dev.daemonPath` to point at another build. A trusted workspace is required to start the daemon and write MCP config.

## Getting Started

1. Trust the workspace when prompted (required for the live editor and MCP).
2. Open any `.op` file — it opens in the OpenPencil canvas editor. Opening a `.fig` file imports it and edits are saved to the sibling `.op`.
3. Run **OpenPencil: Configure MCP** from the Command Palette to register the local MCP proxy with your AI tooling.
4. Run **OpenPencil: Generate Code** to produce React or Vue code from the current design, or chat with **@openpencil** in the VS Code chat view.

## Settings

- `openpencil.dev.daemonPath` — path to the `op-host-web-server` daemon binary (empty probes `target/debug`).
- `openpencil.proxy.port` — port for the local MCP proxy (`0` auto-selects).
- `openpencil.codegen.framework` — target framework for generated code (`react` or `vue`).

## Learn More

Part of the [OpenPencil](https://github.com/ZSeven-W/openpencil) project — an open-source, AI-native, design-as-code vector tool.

## License

MIT
