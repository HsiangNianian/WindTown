# Development guide

A small, native Rust + Bevy multiplayer pixel town. Walk past the coffee shop,
meet your friends, and press **Enter** to talk. All interface text is English;
the pixel font is bundled, so no system font installation is required.

## Run

```sh
cargo run --locked
```

The first build downloads and compiles dependencies. Development builds are
optimized for gameplay. Assets load from an `assets/` folder beside the executable,
or fall back to this source checkout during development. To share a native build,
copy its binary and the entire `assets/` directory into the same folder.

### Main menu

- **Host a room**: choose **Local network** or **Online server**. Online hosting
  asks for a room name (up to 24 characters) and uses the built-in public server.
- **Join LAN**: automatically discovers nearby rooms. Click a room, or enter an
  IP and port yourself. On the same computer, use `127.0.0.1:4761`.
- **Join server**: automatically lists rooms at the selected server. Click a room,
  or enter its eight-character invite code. You can replace the server address.

The lobby refreshes every eight seconds and also has a **Refresh rooms** button.
Online rooms display their chosen name and current player count; LAN rooms use
the host's nickname. Empty online rooms
disappear from the lobby. The **Copy invite** button copies a LAN address or an
online room code; custom servers also need their address shared with friends.

The default deployed server is:

```text
wss://wind-town-multiplayer.opensource-941.workers.dev
```

### Controls

| Input | Action |
| --- | --- |
| A / D or Left / Right | Walk |
| Shift | Run |
| Space | Jump |
| Enter | Open chat / send |
| Escape | Close chat, unfocus a field, or leave the room |
| Tab | Next input field |
| Ctrl+A / Ctrl+V / Ctrl+C | Select, paste, copy in an input |
| 1 / 2 / 3 | Main menu shortcuts |
| 1 / 2 | LAN / server while choosing where to host |
| F11 | Fullscreen |
| F12 | Save an actual game screenshot under `artifacts/` |

Chat pauses movement while typing. Messages appear above the speaker for eight
seconds and in the room's recent chat log. Long bubbles are shortened; the log
retains the full message. Text input supports Unicode and IME composition.

## Networking

**LAN:** the game hosts its own Rust WebSocket server on TCP 4761 (configurable in
the host screen). UDP 4762 handles automatic discovery. Permit these ports on the
host's firewall. Discovery is limited to the same broadcast network; on routed
networks, VPNs, or Wi-Fi with client isolation, use the manual address. One LAN
host per computer uses the discovery port; multiple client windows are supported.
The LAN room closes when its host leaves.

**Online:** a Cloudflare Worker forwards each room to its own SQLite-backed
Durable Object. Hibernating WebSockets retain player attachments, and a separate
Lobby Durable Object lists active rooms and removes stale entries. The room
survives the original host leaving as long as another player remains. Empty rooms
close; chat history and positions are not saved between sessions.

Both transports use the same JSON message protocol. Positions are transmitted
at up to 20 Hz only when changed; remote players interpolate between updates.
Connections have timeouts and heartbeats. Each room allows 16 players, names are
limited to 12 characters, and chat to 80 characters. The servers bound message
size and rate, sanitize text, clamp positions, and assign identities themselves.
This is a friendly social toy: movement is client-driven, not competitive anti-cheat.
The demo's online lobby is limited to 40 simultaneous rooms.

Online connections honor `https_proxy` / `HTTPS_PROXY` and `all_proxy` /
`ALL_PROXY` for unauthenticated HTTP CONNECT proxies, with `no_proxy` / `NO_PROXY`
exceptions. LAN connections always connect directly. TLS certificate validation
remains enabled. Authenticated and SOCKS proxies are not currently supported.
Initial transport failures retry up to three times before showing an error;
room creation requests are never automatically repeated.

Rooms appear publicly in their server's lobby. There are no user accounts,
passwords, moderation tools, matchmaking across unrelated servers, or automatic
reconnection; after a disconnect, return to the menu and join again. Treat LAN
rooms as trusted-network play. Cloudflare account quotas and usage charges apply.

## Cloudflare server

Wrangler is already installed on this machine. On another machine run `npm ci`
inside `server` first, then authenticate with `wrangler login`.

```sh
cd server
npm run dev
```

Set the game's server address to `ws://127.0.0.1:8787` for local Worker testing.

```sh
cd server
wrangler deploy --dry-run
npm run deploy
```

After deploying your own Worker, use its `wss://...workers.dev` address in the menu.
`assets/server-url.txt` supplies the compiled default. `/health` is the health
endpoint, `/rooms` returns the current lobby, `/lobby` serves the same listing
over WebSocket, and `/room/ABCDEFGH` is a room connection.

## Checks

```sh
cargo fmt --all -- --check
cargo test --locked
# Against a running local Worker:
cd server && npm test
# Against the deployed Worker (Node 24+):
SERVER_URL=wss://wind-town-multiplayer.opensource-941.workers.dev node --use-env-proxy test/rooms.mjs
```

The Rust tests cover movement, jumping, map bounds, actual Bevy keyboard event
delivery, Unicode editing, bubble limits, UDP discovery, two real LAN sockets,
and bounded TLS failure handling.
The Worker test covers discovery, lobby cleanup, room isolation, identity, movement,
Unicode chat, disconnects, missing rooms, and oversize input.

To exercise the exact Rust client's WSS, proxy, hosting, lobby, joining, movement,
chat, and disconnect path against a real Worker, from the project root:

```sh
WIND_TOWN_TEST_SERVER=wss://wind-town-multiplayer.opensource-941.workers.dev \
  cargo test --locked cloud_client -- --ignored --nocapture
```

## GPU acceptance and artwork

For a full native GPU acceptance run, launch two debug builds with
`WIND_TOWN_SMOKE=host-lan` and `WIND_TOWN_SMOKE=guest-lan` (or `host-cloud` and
`guest-cloud`). They exercise the actual menu actions, discover and join a room,
inject Bevy keyboard events for walking/jumping/chat, assert movement and both
chat deliveries, save screenshots, then close. This opt-in driver bypasses only
window focus gating; it never sends keys to other desktop applications.

- Bevy **0.18.1** and [bevy_ecs_tilemap **0.18.1**](https://github.com/StarArawn/bevy_ecs_tilemap).
- A 480 × 270 world render texture, nearest-neighbor sampling, and integer pixel
  scaling; high-resolution UI still uses the bundled pixel font.
- Original 16-pixel terrain tiles, four 24 × 32 characters with six animation
  frames each, layered scenery, drifting clouds, and fireflies.
- [Fusion Pixel Font](https://github.com/TakWolf/fusion-pixel-font), 12px monospaced
  Latin variant, release 2026.09.25. Font and upstream licenses are in `assets/fonts/`.
- WebSocket architecture follows Cloudflare's
  [Durable Object hibernation API](https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/).

The checked-in PNGs are ready to run. To regenerate the original art:

```sh
uv run --with Pillow tools/draw_assets.py
```

Native GPU gameplay is verified on Linux. GitHub Actions builds and tests Windows,
Linux, and both macOS architectures. GUI play on Windows/macOS and sessions between
two separate physical LAN machines still need manual verification.

## License

Project code and original artwork: [AGPL-3.0-only](../LICENSE.md).
The bundled font and its upstream notices retain their licenses in `assets/fonts/`.
