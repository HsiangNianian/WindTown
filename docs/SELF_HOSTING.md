# Host your own Yapshire town

[简体中文](SELF_HOSTING.zh-CN.md) · [README](../README.md)

`yapshire-server` runs without a game window, GPU, Node.js or Cloudflare account.
Use it on a home computer, VPS, or Docker host. Players choose its address in
**Host a room → Online server** or **Join server**. Both screens share the address,
which is saved after a connection attempt. **Use official server** restores the
built-in service and clears the current password.

## Start with the standalone download

Download the **yapshire-server** archive for your system from
[Releases](https://github.com/HsiangNianian/Yapshire/releases/latest). Builds are
available for Windows x64, Linux x64 (glibc 2.35+), and macOS Intel / Apple Silicon.
Extract the entire archive. In a terminal inside that folder, run:

```sh
./yapshire-server
```

On Windows, run `./yapshire-server.exe` in PowerShell. The macOS server is a console executable,
not an `.app`. The downloads are unsigned. A server does not need the game's
desktop libraries; Linux needs glibc and the usual C runtime.

The archive includes `server.json` and a `maps/` directory. The server reads
`server.json` in its working directory automatically. Without that file, the
executable runs its embedded original town, so it also works on its own.

The default town is **My Yapshire town**, code **MAIN0001**, listening on
**TCP 4761**. On the same computer, enter `ws://127.0.0.1:4761` in the game's
**Join server** screen, then choose that town from the lobby. On your local
network, use the server computer's LAN address, such as `ws://192.168.1.10:4761`.
The dedicated server uses **Join server**; automatic **Join LAN** discovery is
for rooms hosted inside the game.

Ctrl+C stops the server and disconnects its players. The configured town remains
listed while empty, and comes back on restart. Extra rooms created from the game
are removed when their last player leaves. All rooms on one server use its maps.

## Docker

The public image supports **linux/amd64** and **linux/arm64**:

```sh
docker run -d --name yapshire --restart unless-stopped \
  -p 4761:4761 ghcr.io/hsiangnianian/yapshire-server:latest
```

Use a version tag such as `:v0.5.0` to pin a release. The repository also includes
[`compose.yaml`](../compose.yaml): `docker compose up -d` starts the default town.
The image runs as UID/GID **10001**, includes an HTTP health check, and reads
configuration from `/data`. It never needs to write to your maps.

To create editable files without a native executable:

```sh
docker run --name yapshire-init ghcr.io/hsiangnianian/yapshire-server:latest --init /tmp/my-town
docker cp yapshire-init:/tmp/my-town ./my-town
docker rm yapshire-init
```

Start a server with those files mounted read-only:

```sh
docker run -d --name yapshire-custom --restart unless-stopped \
  -p 4761:4761 --read-only --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --mount "type=bind,src=$PWD/my-town,dst=/data,readonly" \
  ghcr.io/hsiangnianian/yapshire-server:v0.5.0
```

Ensure the directory and files are readable by UID 10001. In Compose, enable the
commented `./my-town:/data:ro` mount. Use a different host port if another game
or server already uses 4761. Keep the container's port at 4761 for its built-in
health check; for a custom internal port, override the health check as well.

## Use maps from the game editor

1. Create a server folder: `./yapshire-server --init ./my-town`. This writes a
   config and both original maps, and refuses to overwrite existing files.
2. In the game, open **Map editor**, make your changes, and **Save**.
3. Click **Map files**. Copy the edited `town.tmj` and/or `tackle-shop.tmj` into
   `my-town/maps/`, replacing only the corresponding map. Keep both map files.
4. Check and start the server:

```sh
./yapshire-server --config ./my-town/server.json --check
./yapshire-server --config ./my-town/server.json
```

The editor, client and server use the **same Tiled JSON `.tmj` format and Rust
validation module**. No export conversion or client rebuild is required. You can
also edit the files in Tiled. Layout changes take effect after a server restart;
players reconnect to receive the new snapshot.

On joining, the server sends both maps and their revision. The client checks map
dimensions, tile IDs, the shared tileset fingerprint and the revision, then
acknowledges that exact world before entering the room. Remote maps stay in
memory. Leaving or losing the connection restores the player's own local maps;
downloaded maps never overwrite editor files.

The supported format is deliberately small:

- Town: **90 × 17**, shop: **30 × 17**, **16 × 16** tiles, five finite orthogonal
  tile layers, uncompressed JSON arrays, original dimensions and zero offsets.
- The bundled **359-tile `harbor.tsj` / `harbor.png`** palette is shared by every
  player. Leave those files unchanged. Custom images, tilesets, scripts, object
  layers, and variable map sizes are not transferred or supported.
- Tile flips, empty tiles, hidden layers and Tiled metadata are preserved.
  Each input `.tmj` is limited to **256 KiB**; a network world is bounded to
  **512 KiB**. Invalid maps fail validation before the server opens a port.
- Collision and interactions remain fixed. The editor's gold guides mark the
  walking surface, shop door, counter and fishing area. Changing artwork does
  not move these anchors; see [Tilemaps](DEVELOPMENT.md#tilemaps).

LAN hosts publish their saved local editor maps through the same server library.
The existing public Cloudflare Worker uses the bundled original map; clients
temporarily switch to that map when joining it. Use **v0.5.0 or later** clients
for the new dedicated server and LAN map handshake (protocol 2). The v0.5 client
also remains compatible with the existing Worker protocol.

## Configuration

All fields are optional. Paths in `server.json` are relative to that config file;
a CLI `--maps` path is relative to the process's working directory.

```json
{
  "bind": "0.0.0.0:4761",
  "name": "My Yapshire town",
  "room_code": "MAIN0001",
  "maps_dir": "maps",
  "max_players": 16,
  "max_rooms": 40,
  "max_connections_per_ip": 32,
  "allow_room_creation": true,
  "allowed_origins": []
}
```

`room_code` is eight uppercase letters or digits, `name` is at most 24 characters,
`max_players` is 1–16, `max_rooms` is 1–40, and `max_connections_per_ip` is 1–256.
Set `allow_room_creation` to `false` to offer only the configured town. Set
`maps_dir` to `null` to use embedded maps. Invalid or unknown options fail early.
`--bind`, `--maps`, `--config`, `--check`, `--init`, `--help`, and `--version` are
available; `--init` is used by itself.

## Optional shared password

Set **YAPSHIRE_SERVER_PASSWORD** to an 8–128 byte password in the server's
environment. Players enter it in the masked **Server password** field. It is
kept only for that game process, never saved in settings or copied with an invite.
Changing the address clears it. Share passwords separately from room codes.

In Bash or zsh, read the password without echoing it or putting it in history:

```sh
read -rs YAPSHIRE_SERVER_PASSWORD
export YAPSHIRE_SERVER_PASSWORD
./yapshire-server --config ./my-town/server.json
```

For Docker, set the variable in the shell first, then pass
`--env YAPSHIRE_SERVER_PASSWORD` to `docker run`. Compose reads the same variable.
Container administrators can inspect environment variables, so protect access
to the Docker host. A password protects both joining and lobby listing. `/health`
remains public and contains only status, version, protocol and map revision.

This is a shared server password, not per-player accounts or moderation. The
wire header is `Authorization: Bearer <SHA-256(password)>`; that digest is itself
a credential. It is never placed in a URL. Use TLS for public connections.

## Internet access and TLS

Permit the chosen TCP port in the server firewall. A home connection also needs
router port forwarding; carrier-grade NAT may require a VPS or a VPN with peer
connectivity. Friends need your server address **and** a room code, plus the
password if enabled. The **Copy invite** button copies the room code only.

For a public domain, bind the server to `127.0.0.1:4761` and terminate TLS at your
reverse proxy. For example, a [Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
can forward WebSockets:

```caddyfile
town.example.com {
    reverse_proxy 127.0.0.1:4761
}
```

Players then enter `wss://town.example.com`. Use a valid certificate, preserve
the `/health`, `/rooms`, `/lobby` and `/room/*` paths, and forward the Authorization
header. With Docker, publish `127.0.0.1:4761:4761` when the proxy runs on the host.

The native client sends no browser Origin. Browser WebSocket origins are rejected
unless explicitly listed in `allowed_origins`. The server intentionally ignores
forwarded IP headers: per-address limits use the actual TCP peer. Behind a proxy,
clients share that proxy's IP budget (300 connection/list requests per minute and
the configured connection limit). For larger deployments, apply limits at the
proxy and plan capacity around this shared budget.

Each socket has bounded messages (2 KiB), rate (80 messages/second), an outgoing
queue, a 10-second map acknowledgement deadline and a 45-second heartbeat
deadline. The server assigns player IDs, clamps movement and cleans text. A slow,
malformed or flooding peer is disconnected without taking down other rooms.
This is a social game with client-driven movement, not competitive anti-cheat.
Wallets and fishing saves remain on each player's computer; positions and chat
history are not persisted by the server.

## Build and verify

```sh
cargo build --release --locked -p yapshire-server
cargo test --locked -p yapshire-server -p yapshire-shared
docker build -t yapshire-server:local .
python3 tools/check_container.py yapshire-server:local
```

The server build does not compile Bevy or require GUI libraries. CI tests real
password-protected connections and edited maps, builds all four native server
archives alongside the clients, and checks both container architectures before
publishing the GitHub release. Versioned GHCR images carry the source revision;
release archives have `SHA256SUMS`.
