# l-gpui

A native, clean-room **Lark Suite desktop client** written in Rust with
[GPUI](https://www.gpui.rs/). l-gpui does not talk to Lark servers itself:
every action is delegated to the official command-line tool
[`lark-cli`](https://www.npmjs.com/package/@larksuite/cli) (`@larksuite/cli`),
which owns configuration, OAuth sign-in and token storage. l-gpui never reads
or prints your tokens.

> Not affiliated with or endorsed by Lark / ByteDance. No Lark code or assets
> are included; the UI is built only from GPUI primitives and original artwork.

## Features

| Module | Status | What works | lark-cli command(s) |
|---|---|---|---|
| Sign-in | ✅ Working | Detects not-installed / not-configured / signed-out / signed-in, shows the setup commands with Copy buttons, re-checks automatically | `auth status --json` |
| Messenger | ✅ Working | Chat list (groups + direct, most recent first, local filter), message history (auto-refresh every 10 s), compose & send text, message search | `im +chat-list`, `im +chat-messages-list`, `im +messages-send`, `im +messages-search` |
| Calendar | ✅ Working | Agenda for today / 7 / 30 days, grouped by day | `calendar +agenda` |
| Tasks | ✅ Working | My open tasks, complete with one click, create, open in Lark | `task +get-my-tasks`, `task +complete`, `task +create` |
| Docs & Wiki | ✅ Working | Document search (falls back to Drive search), open in browser, list Wiki spaces | `docs +search`, `drive +search`, `wiki +space-list` |
| Mail | ✅ Working | Inbox triage + search, read plain-text body, compose / reply and send | `mail +triage`, `mail +message`, `mail +send --confirm-send` |
| Contacts | ✅ Working | People search, "Email" shortcut into Mail compose | `contact +search-user` |
| Approval | ✅ Read-only | Pending and handled approval tasks, open in Lark to act | `approval tasks query` |
| Minutes | ✅ Read-only | Search / list minutes, open in browser | `minutes +search` |
| Meetings | 🚧 Stub | "Coming soon" screen | — |
| OKR | 🚧 Stub | "Coming soon" screen | — |
| Base | 🚧 Stub | "Coming soon" screen | — |
| Attendance | 🚧 Stub | "Coming soon" screen | — |

Keyboard: click a field to type; **Enter** submits (sends a message, runs a
search, adds a task), **Shift+Enter** inserts a newline, **Tab** moves between
mail fields, **Ctrl+Enter** sends mail, **Ctrl/Cmd+V** pastes,
**Ctrl/Cmd+R** refreshes the current view, **Esc** leaves a field.

## Install

### 1. Install lark-cli (required)

l-gpui needs Node.js (18+) and the official CLI:

```sh
npm i -g @larksuite/cli
lark-cli --version
```

### 2. Install l-gpui

Download the `.deb` from the [latest release](https://github.com/zeroexlive/l-gpui/releases/latest):

```sh
sudo apt install ./l-gpui_0.1.0-1_amd64.deb
```

or use the `l-gpui-<version>-linux-x86_64.tar.gz` archive and run `./l-gpui`.
Verify downloads with `sha256sum -c SHA256SUMS.txt`.

The app needs a GPU with Vulkan support plus X11 or Wayland
(`libvulkan1`, `libxkbcommon-x11-0`, `libwayland-client0`; the `.deb` pulls these in).

### 3. Sign in

Run these once in a terminal (each prints a link to open in your browser):

```sh
lark-cli config init --new          # create/attach a Lark app for the CLI
lark-cli auth login --recommend     # sign in as yourself with the recommended scopes
```

Then start **l-gpui** from your app menu. If you open it before signing in it
shows these steps and switches to the main window by itself once
`lark-cli auth status` reports a signed-in identity.

### Environment variables

| Variable | Meaning |
|---|---|
| `LARK_CLI` | Path to the lark-cli executable (default: `lark-cli` on `PATH`) |
| `LARK_CLI_AS` | Identity passed as `--as` to every data command: `user` or `bot` |

## Build from source

```sh
# Linux build deps (Debian/Ubuntu)
sudo apt install pkg-config libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libx11-xcb-dev libxcb1-dev libfontconfig-dev libfreetype-dev \
  libssl-dev libzstd-dev libasound2-dev cmake clang

cargo test --no-default-features   # core library + fake-CLI integration tests (no GPU needed)
cargo run --release                # the GPUI app
cargo install cargo-deb && cargo deb   # build a .deb
```

The `ui` feature (on by default) pulls in GPUI. `--no-default-features` builds
only the `lgpui_core` library, which is what the unit tests exercise.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how it fits together.

## Known limitations

* Messages show text content; images, files and cards appear as the
  placeholders lark-cli renders.
* Approvals and Minutes are read-only (open them in Lark to act).
* No notifications or tray icon yet; Messenger polls every 10 seconds.
* Text fields are minimal (append / backspace / paste, no cursor movement or IME).

## License

MIT, see [LICENSE](LICENSE).
