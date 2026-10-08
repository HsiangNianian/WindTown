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
| F11 | Toggle fixed-size window / fullscreen |
| F12 | Save an actual game screenshot under `artifacts/` |

Chat pauses movement while typing. Messages appear above the speaker for eight
seconds and in the room's recent chat log. Long bubbles are shortened; the log
retains the full message. Text input supports Unicode and IME composition.

Windowed mode is fixed at 1440 × 810 logical pixels; resizing and maximizing are
disabled. F11 switches to borderless fullscreen on the current monitor and
restores the fixed size when leaving. The UI and world share a centered 16:9
viewport and integer pixel scale, including on HiDPI and ultrawide displays.

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

Run `npm ci` inside `server` to install the project's pinned Wrangler version.
Authenticate with `npx wrangler login` when deploying to your own account.
The project uses the official npm registry. A temporary `sharp` 0.35.5 override
patches Miniflare's pinned image dependency; remove it when Miniflare includes
that fix upstream.

```sh
cd server
npm run dev
```

Set the game's server address to `ws://127.0.0.1:8787` for local Worker testing.

```sh
cd server
npx wrangler deploy --dry-run
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

`WIND_TOWN_SMOKE=display cargo run --locked` checks a fixed window, F11 fullscreen,
and the restored window size, saving screenshots of all three stages under
`artifacts/`. This uses the real window backend and GPU, with no desktop input injection.

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

## Cross-platform CI

The workflow layout follows [IntelligentMixVideo](https://github.com/HsiangNianian/IntelligentMixVideo):
a reusable build matrix, separate checks, and a tag-triggered release workflow.

| Event | Result |
| --- | --- |
| Main branch push / pull request | Project checks, native tests and four platform archives |
| Documentation-only push / PR | Build skipped |
| Manual **CI** | Full checks and four platform archives |
| Manual **Build game** | Four platform archives |
| `vX.Y.Z` tag | Version checks, tests, four platform archives, verified Release and CHANGELOG update |

All builds use lockfiles. Actions artifacts are retained for 14 days. Both CI and
releases call the same build workflow; release binaries come from the tagged
commit. Normal jobs use read permissions; only the release publishing job can
write repository contents.

Local checks:

```sh
cargo fmt --all -- --check
cargo test --locked
node --test .github/scripts/*.test.mjs
python3 -m unittest discover -s tools -p 'test_*.py'
# With a local Worker already running:
npm test --prefix server
```

## Releases and changelog

Game and Worker versions move together. Prepare a version from the repository
root, review the diff, and commit it before tagging:

```sh
RELEASE_TAG=v0.1.0 node .github/scripts/validate-release.mjs --write
node .github/scripts/validate-release.mjs
git add Cargo.toml Cargo.lock server/package.json server/package-lock.json
git commit -m "chore: prepare v0.1.0"
# Once the release commit is on main:
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin main v0.1.0
```

For an unchanged first version, skip the empty version commit. PowerShell users
can set `$env:RELEASE_TAG = "v0.1.0"` before running the same Node command.
Only stable `vX.Y.Z` tags are accepted. CI rejects source/tag version mismatches;
it never changes source versions during a release.

[`release.yml`](../.github/workflows/release.yml) generates notes from Conventional
Commits using the same changelog action as IntelligentMixVideo. The range runs
from the preceding ancestor version tag to the new tag. The first release uses
the empty repository bootstrap commit as its baseline.

After every build succeeds, the workflow uploads the four archives,
`SHA256SUMS`, and `CHANGELOG.md` to a **draft**. It verifies the uploaded names,
sizes and available digests before publishing. Release Notes and the changelog
entry come from the same generated changes. Then a bot merges that entry into
the latest default branch, preserving concurrent edits and existing releases.

Failed drafts can be retried. Published releases remain unchanged; a retry can
repair changelog writeback using the already published attachment. No personal
token is required: the built-in `GITHUB_TOKEN` handles publishing. Branch rules
must allow its changelog commit. Tagged releases do not deploy the Worker.

## License

Project code and original artwork: [AGPL-3.0-only](../LICENSE.md).
The bundled font and its upstream notices retain their licenses in `assets/fonts/`.
