# Notion GPUI

A native, local-first desktop workspace scaffold for **Windows, macOS, and Linux**,
built with [GPUI Kit](https://github.com/longbridge/gpui-kit). Includes a
Notion-style team task board and AI chat.

## What works

- Create tasks with an optional owner, move them through Backlog / In progress /
  Done, and delete them.
- Persist the board and selected page in the OS user-data directory with atomic
  writes. Saving runs in the background; failures remain visible in the app.
- Chat with OpenAI when configured, or explore a clearly labeled offline demo.
  Requests run off the UI thread, have a 60-second timeout, and surface errors
  without exposing provider response bodies or credentials.
- Navigate between the board and chat without losing the current session.

**Scope:** this is a foundation, not a production collaboration service. The board
is local, not connected to the Notion API. Owners are labels, not authenticated
team members. Notion OAuth, shared workspaces, realtime synchronization,
conflict resolution, installers, auto-updates, and a production credential vault
are future integration work. Do not run two instances against the same workspace.
Browser/WASM builds are not supported by this desktop scaffold.

## Develop on Windows

1. Install [Rust](https://rustup.rs/) using the default **MSVC** toolchain.
2. Install **Visual Studio 2022 Build Tools**, selecting **Desktop development
   with C++**, the MSVC toolset, and a Windows 10/11 SDK (including UCRT and the
   resource compiler).
3. Open **Developer PowerShell for VS 2022** in the repository directory:

   ```powershell
   rustup update stable
   cargo run --locked
   ```

Use PowerShell rather than Git Bash to avoid its `link.exe` shadowing the MSVC
linker. If the linker or `rc.exe` is missing, check the workload/SDK installation
and restart Developer PowerShell. The initial native build is substantial.

### macOS

Install Xcode command-line tools (`xcode-select --install`) and Rust, then run
`cargo run --locked`.

### Linux (Ubuntu 24.04)

Install Rust and the native GPUI build dependencies:

```sh
sudo apt-get update
sudo apt-get install -y build-essential clang pkg-config \
  libfontconfig-dev libfreetype-dev libwayland-dev wayland-protocols \
  libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libssl-dev libvulkan-dev libvulkan1
cargo run --locked
```

A graphical X11/Wayland session and Vulkan-capable graphics driver are required
at runtime. CI checks compilation and non-graphical unit tests, not interactive
rendering on each OS.

## AI chat

Without **both** variables below, chat uses a deterministic local demo, **not an
AI model**. For real chat, set them in the launching shell:

```powershell
$env:OPENAI_API_KEY = Read-Host "OpenAI API key" -MaskInput
$env:OPENAI_MODEL = "your-enabled-chat-completions-model"
cargo run --locked
```

`-MaskInput` requires PowerShell 7.1+. On other shells, use your environment or
secret manager to supply the key without putting it in shell history.

Choose a model in your account that supports Chat Completions and
`max_completion_tokens`. Clicking **Send** shares this session's chat history
with `https://api.openai.com/v1/chat/completions` and may incur charges. The
adapter requests `store: false`; provider retention policies still apply.
The app never sends board contents automatically. Chat history is memory-only
and **Clear chat** removes it. The draft remains available for editing/retry.
Keys are read at startup and are never serialized or logged. `.env` files are
ignored by Git but are **not automatically loaded**.

## Architecture

Inspired by [GPUI Starter's architecture guide](https://github.com/freeoxide/gpui-starter/blob/master/docs/architecture.md),
without copying its gallery or importing its unrelated services and git forks.

| Layer | Responsibility |
| --- | --- |
| `src/main.rs`, `src/app.rs` | Native entry, component initialization, window and lifecycle |
| `src/state/` | Versioned workspace snapshot, GPUI singleton, ordered background saves |
| `src/features/board.rs` | Board model, task validation and transitions |
| `src/features/chat.rs` | Transient conversation and request state |
| `src/services/ai.rs` | Provider boundary, demo and HTTPS OpenAI adapter |
| `src/services/storage.rs` | Atomic JSON persistence and corrupt-file quarantine |
| `src/shell/` | Entity-owned root, route navigation, board and chat UI |
| `src/platform/` | OS-specific user-data location |

Initialization loads the workspace before entering GPUI, initializes Kit before
using components, installs global state, and opens an `Entity<AppRoot>` through
Kit's root-wrapping window helper. State mutations go through `state::update`;
observers notify the view. Saves are serialized to prevent older snapshots
overwriting newer ones, and the quit hook awaits pending writes within GPUI's
shutdown grace period. Views use `cx.notify()`; provider work uses the background
executor and a weak entity update, allowing the window to close safely.

The UI foundation is pinned to **GPUI Kit 0.8.0**, the current published release
when scaffolded, which uses **gpui-pre 0.3.9**. The supplied guide describes Kit
0.7.3. Depend on Kit's re-exports rather than independently adding `gpui 0.2.x`;
this keeps one GPUI snapshot/component/assets family. `gpui-pre` is a
maintainer-published Zed snapshot, not an official Zed crates.io release.
`Cargo.lock` records the complete desktop dependency graph; there are no vendored
sources, local path overrides, or starter-specific form/query forks.

Workspace file locations:

- Windows: `%LOCALAPPDATA%\notion-gpui\notion-gpui\data\workspace.json`
- macOS: `~/Library/Application Support/dev.notion-gpui.notion-gpui/workspace.json`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/notion-gpui/workspace.json`

Invalid JSON is preserved as `workspace.json.bad` before defaults are loaded.
An existing recovery file, unreadable storage, or an unsupported schema stops
startup instead of overwriting data. Back up this plaintext workspace as needed;
it is not an encrypted credential store.

## Validate

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

Tests cover task validation/transitions, chat concurrency/error recovery, provider
response parsing, persistence round trips, and corrupt/future-schema recovery.
The CI matrix builds and runs tests on Windows, macOS, and Ubuntu.

To update dependencies, use `cargo update`, review the lockfile and advisories,
and verify `cargo tree -i gpui-pre` / `cargo tree -i gpui-kit` still each resolve
to one version before running the checks above on all three platforms.
