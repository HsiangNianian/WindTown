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
wss://yapshire-multiplayer.opensource-941.workers.dev
```

### Controls

| Input | Action |
| --- | --- |
| A / D or Left / Right | Walk |
| Shift | Run |
| Space | Jump / hook a bite / hold to reel |
| E | Enter / leave the tackle shop, browse the counter, cast at the pier |
| I | Open / close the illustrated satchel |
| Enter | Open chat / send |
| Escape | Close a panel / cancel a cast, exit the shop, close chat, unfocus a field, or leave the room |
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

## Fishing and local progress

Version 0.3.0 includes a walkable tackle-shop interior and a coastal pier.
New nicknames start with 100 coins. The counter sells a rod (45), a reusable hook
(15), and five worms (10); both tackle items equip automatically. Each cast spends
one worm, including missed bites, escaped fish and cancelled casts. A bite gives
1.5 seconds to press Space. Hold Space to reel, release to reduce tension, and
bring the catch meter to full. Fish can be sold at the counter. If a player with
tackle runs out of bait, has no fish, and cannot afford a bait pack, the counter
provides one emergency worm.

The satchel shows pixel-art tackle, a worm tin, coins and four fish slots, with
equipped states and stack counts. The shop uses matching illustrated purchase
cards. At the end of the pier, casting animates the rod and line into open water;
the float bobs, a bite splashes, and the fish darts as it pulls against the line.
The small water view shows the fish moving closer as you reel. A landed fish
lifts out of the sea before appearing in the satchel. The chat and controls
panels hide during these activities so the float remains visible.

Purchases, casts, catches and sales save automatically per nickname. Files are
named with the nickname's UTF-8 bytes encoded as hex:

| Platform | Save directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/yapshire`, or `~/.local/share/yapshire` |
| Windows | `%APPDATA%/Yapshire` |
| macOS | `~/Library/Application Support/Yapshire` |

`YAPSHIRE_SAVE_DIR` overrides the directory for isolated tests. Writes replace
the save through a temporary file. Invalid or unsupported saves disable purchases
and fishing instead of overwriting the file; an on-screen message reports load or
write failures. Keep using the same nickname to resume progress.

Progress is local to each computer. Coins and catches are not traded or synchronized
between players. Movement packets synchronize the shop area and fishing pose;
both the LAN relay and Worker accept older packets with these optional flags absent.
Update a self-hosted Worker together with the client to show the new area and poses.

## Tilemaps

The street floor, quay, animated water, timber pier, tackle-shop exterior and
interior render through `bevy_ecs_tilemap`. The coastal area shares the town's
sky and hills. The older street's decorative buildings and trees remain a
background illustration; characters, signs above NPCs and fishing rods are sprites.

Open these files directly in [Tiled](https://www.mapeditor.org/):

| File | Contents |
| --- | --- |
| `assets/maps/town.tmj` | 90 × 17 cells: water, shore and pilings, terrain, buildings, props |
| `assets/maps/tackle-shop.tmj` | 30 × 17 cells: backdrop, walls, floor, furniture, counter |
| `assets/maps/harbor.tsj` | Shared 16 × 16 tiles and water animations |
| `assets/maps/harbor.png` | The tileset image |

Maps load from the same runtime `assets/` folder as the artwork. Save a map and
restart the game to see layout changes; rebuilding Rust is unnecessary. Keep
the existing five layers in order, their original dimensions and offsets, and
use uncompressed JSON tile arrays. Empty cells, hidden layers and Tiled tile
flips are supported. Invalid sizes, tile IDs and animation ranges are rejected
at startup instead of producing a broken tilemap. This is a small loader for
these finite orthogonal maps, not a general importer for every Tiled feature.

The current demo has one flat walking surface at world **y = 0**, corresponding
to **y = 208** in Tiled (the top of tile row 13). Editing artwork does not change
collision or interaction positions: the outdoor shop door is x = 965, indoor
exit x = 64, counter x = 270, and fishing begins at x = 1304. The pier ends at
x = 1360; players stop 12 pixels before its edge and cast into the water beyond.
Preserve these anchors when editing. New platforms, slopes or moved interactions need matching
gameplay changes; custom maps are not synchronized between multiplayer clients.

To regenerate the original harbor tiles and both default layouts:

```sh
uv run --with Pillow tools/draw_maps.py
```

This **overwrites** the map layouts, so keep any hand-edited maps first. The
generator checks tile bounds and a continuous floor. Rust tests additionally
check map loading, interaction alignment and invalid tile data. Release packaging
requires all four map assets and the four `assets/fishing/` images on every platform.

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
SERVER_URL=wss://yapshire-multiplayer.opensource-941.workers.dev node --use-env-proxy test/rooms.mjs
```

The Rust tests also cover tackle purchases, insufficient funds, repeated purchases,
bait consumption, selling, save replacement and validation, and fishing wins/losses
at 30, 60 and 144 simulation steps per second.
The Rust tests cover movement, jumping, map bounds, actual Bevy keyboard event
delivery, Unicode editing, bubble limits, UDP discovery, two real LAN sockets,
and bounded TLS failure handling.
The Worker test covers discovery, lobby cleanup, room isolation, identity, movement,
shop and fishing flags, invalid activity input,
Unicode chat, disconnects, missing rooms, and oversize input.

To exercise the exact Rust client's WSS, proxy, hosting, lobby, joining, movement,
chat, and disconnect path against a real Worker, from the project root:

```sh
YAPSHIRE_TEST_SERVER=wss://yapshire-multiplayer.opensource-941.workers.dev \
  cargo test --locked cloud_client -- --ignored --nocapture
```

## GPU acceptance and artwork

On Linux, run the complete fishing loop with a fresh, isolated save directory:

```sh
YAPSHIRE_SMOKE=fishing YAPSHIRE_SAVE_DIR="$(mktemp -d)" cargo run --locked
# With the local Worker running:
YAPSHIRE_SMOKE=fishing-cloud YAPSHIRE_TEST_SERVER=ws://127.0.0.1:8787 \
  YAPSHIRE_SAVE_DIR="$(mktemp -d)" cargo run --locked
```

This debug-only driver uses the real game controls to walk into the shop, buy
tackle and bait, reach the pier, miss one bite, catch a fish by controlling line
tension, return to the counter, sell it, and verify the saved balance. It saves
actual screenshots, including the street-to-shop and quay-to-pier transitions, under `artifacts/fishing-*.png` or
`artifacts/fishing-cloud-*.png`. Use a fresh save directory for every run.
The LAN run uses local port 4777.

`YAPSHIRE_SMOKE=display cargo run --locked` checks a fixed window, F11 fullscreen,
and the restored window size, saving screenshots of all three stages under
`artifacts/`. This uses the real window backend and GPU, with no desktop input injection.

For a full native GPU acceptance run, launch two debug builds with
`YAPSHIRE_SMOKE=host-lan` and `YAPSHIRE_SMOKE=guest-lan` (or `host-cloud` and
`guest-cloud`). They exercise the actual menu actions, discover and join a room,
inject Bevy keyboard events for walking/jumping/chat, assert movement and both
chat deliveries, save screenshots, then close. This opt-in driver bypasses only
window focus gating; it never sends keys to other desktop applications.

To record the same real session for README media, set
`YAPSHIRE_RECORD=artifacts/recording` on the host process. The debug driver saves
15 FPS PNG frames while the two players walk, jump, and chat. Use a new empty
directory for each recording. For example, launch these in separate terminals:

```sh
YAPSHIRE_SMOKE=host-cloud YAPSHIRE_RECORD=artifacts/recording cargo run --locked
YAPSHIRE_SMOKE=guest-cloud cargo run --locked
```

The English and Chinese READMEs use the same captured gameplay, with localized
captions outside the game viewport. The game interface remains English. Banner
lettering is stored as SVG paths, so Chinese text needs no installed fonts.

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
uv run --with Pillow tools/draw_fishing.py
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
root, update both README download tables and add the version entry to
`CHANGELOG.md`, then review and commit the changes before tagging:

```sh
RELEASE_TAG=v0.3.0 node .github/scripts/validate-release.mjs --write
node .github/scripts/validate-release.mjs
git add Cargo.toml Cargo.lock server/package.json server/package-lock.json
git add README.md README.zh-CN.md CHANGELOG.md
git commit -m "chore: prepare v0.3.0"
# Once the release commit is on main:
git tag -a v0.3.0 -m "Release v0.3.0"
git push origin main v0.3.0
```

For an unchanged first version, skip the empty version commit. PowerShell users
can set `$env:RELEASE_TAG = "v0.3.0"` before running the same Node command.
Only stable `vX.Y.Z` tags are accepted. CI rejects source/tag version mismatches;
it never changes source versions during a release.

[`release.yml`](../.github/workflows/release.yml) reads Release Notes directly
from the tagged changelog entry. Preparing the entry before tagging keeps the
notes, uploaded changelog and documentation inside every archive identical.
If the entry is absent, the workflow generates it from Conventional Commits
using the same changelog action as IntelligentMixVideo; that fallback entry
cannot appear in archives already built from the tag. The generated range runs
from the preceding ancestor version tag to the new tag. The first release uses
the empty repository bootstrap commit as its baseline.

After every build succeeds, the workflow uploads the four archives,
`SHA256SUMS`, and `CHANGELOG.md` to a **draft**. It verifies the uploaded names,
sizes and available digests before publishing. Release Notes and the changelog
entry use the same text. Then a bot merges any missing entry into
the latest default branch, preserving concurrent edits and existing releases.

Failed drafts can be retried. Published releases remain unchanged; a retry can
repair changelog writeback using the already published attachment. No personal
token is required: the built-in `GITHUB_TOKEN` handles publishing. Branch rules
must allow its changelog commit. Tagged releases do not deploy the Worker.

## License

Project code and original artwork: [AGPL-3.0-only](../LICENSE.md).
The bundled font and its upstream notices retain their licenses in `assets/fonts/`.
