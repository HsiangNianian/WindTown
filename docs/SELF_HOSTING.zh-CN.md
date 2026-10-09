# 开自己的 Yapshire 小镇

[English](SELF_HOSTING.md) · [返回首页](../README.zh-CN.md)

`yapshire-server` 是独立的无界面服务端，不需要显卡、Node.js 或 Cloudflare 账号，
可以运行在家用电脑、VPS 或 Docker 中。玩家在**创建房间 → 在线服务器**或
**服务器联机**中填写地址。这两个页面共用地址，发起连接后会保存，重启游戏也会记住。
**使用官方服务器**按钮恢复内置地址，同时清空本次输入的密码。

## 用独立程序开服

在 [Releases](https://github.com/HsiangNianian/Yapshire/releases/latest) 下载对应系统的
**yapshire-server** 压缩包：Windows x64、Linux x64（glibc 2.35+）、macOS Intel 或
Apple Silicon。完整解压，在该文件夹打开终端：

```sh
./yapshire-server
```

Windows PowerShell 使用 `./yapshire-server.exe`。macOS 服务端是终端程序，不是 `.app`。
下载包暂未签名。服务端不需要游戏的桌面依赖，Linux 需要 glibc 和常规 C 运行库。

压缩包包含 `server.json` 和 `maps/`。程序自动读取当前工作目录下的 `server.json`；
没有配置文件时，使用程序内嵌的原版地图，因此单独复制可执行文件也可以启动。

默认小镇名为 **My Yapshire town**，邀请码 **MAIN0001**，监听 **TCP 4761**。
同一台电脑的玩家在游戏的**服务器联机**中输入 `ws://127.0.0.1:4761`，点击大厅里的小镇。
局域网朋友则填写服务器电脑的地址，例如 `ws://192.168.1.10:4761`。
独立服务端通过**服务器联机**进入；**局域网联机**的自动发现用于游戏内创建的局域网房间。

按 Ctrl+C 停止服务端，在线玩家会断开。配置文件里的小镇在没有玩家时仍然存在，
重启后也会重新出现。玩家在游戏里额外创建的房间会在最后一位玩家离开后移除。
同一个服务端上的所有房间使用该服务端的地图。

## Docker 开服

公开镜像支持 **linux/amd64** 和 **linux/arm64**：

```sh
docker run -d --name yapshire --restart unless-stopped \
  -p 4761:4761 ghcr.io/hsiangnianian/yapshire-server:latest
```

可以用 `:v0.5.0` 等版本标签固定版本。仓库提供了 [`compose.yaml`](../compose.yaml)，
运行 `docker compose up -d` 即可启动默认小镇。
镜像以 UID/GID **10001** 运行，内置 HTTP 健康检查，从 `/data` 读取配置，不会写入地图。

没有原生服务端程序时，也能用镜像生成可编辑文件：

```sh
docker run --name yapshire-init ghcr.io/hsiangnianian/yapshire-server:latest --init /tmp/my-town
docker cp yapshire-init:/tmp/my-town ./my-town
docker rm yapshire-init
```

将文件以只读方式挂载后启动：

```sh
docker run -d --name yapshire-custom --restart unless-stopped \
  -p 4761:4761 --read-only --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --mount "type=bind,src=$PWD/my-town,dst=/data,readonly" \
  ghcr.io/hsiangnianian/yapshire-server:v0.5.0
```

目录和文件需要允许 UID 10001 读取。使用 Compose 时，取消 `./my-town:/data:ro` 挂载行的注释。
如果 4761 已被游戏或另一台服务端占用，可以换宿主机端口。容器内部保持 4761，健康检查即可
直接使用；修改内部监听端口时，也需要同步覆盖健康检查地址。

## 使用游戏编辑器的地图

1. 运行 `./yapshire-server --init ./my-town`，生成配置和两张原版地图。初始化不会覆盖已有文件。
2. 在游戏中打开**地图编辑器**，完成修改并**保存**。
3. 点击**地图文件**，将编辑后的 `town.tmj` 和／或 `tackle-shop.tmj` 复制到 `my-town/maps/`，
   仅替换对应地图，保留另一张地图。服务端需要两张文件都存在。
4. 检查并启动：

```sh
./yapshire-server --config ./my-town/server.json --check
./yapshire-server --config ./my-town/server.json
```

编辑器、客户端和服务端使用**同一套 Tiled JSON `.tmj` 格式与 Rust 校验模块**，
不需要转换或重新编译客户端。也可以直接用 Tiled 编辑。修改文件后重启服务端，
玩家重新连接即可收到新地图。

加入时，服务端下发两张地图及内容版本，客户端校验尺寸、图块编号、图块集指纹和内容版本，
确认完全一致后才进入房间。远程地图只保留在内存里，离开或断线后恢复玩家自己的本地地图，
不会覆盖编辑器保存的文件。

目前支持的地图范围：

- 小镇 **90 × 17**、渔具店 **30 × 17**，**16 × 16** 像素图块，五层有限正交 tile layer，
  未压缩 JSON 数组，保持原尺寸和零偏移。
- 共用随游戏发布的 **359 个图块**，请保持 `harbor.tsj` 和 `harbor.png` 不变。
  暂不传输或支持自定义图片、图块集、脚本、对象层及不同尺寸的地图。
- 保留空图块、翻转、图层显隐与 Tiled 元数据。每张输入 `.tmj` 上限 **256 KiB**，
  整个网络地图消息上限 **512 KiB**。无效地图会在开放监听端口之前报错。
- 碰撞和交互仍固定；编辑器中的金色辅助线标出了行走面、门、柜台和垂钓区。
  修改美术不会移动这些位置，详见[地图说明](DEVELOPMENT.md#tilemaps)。

游戏内局域网房主也会通过同一服务端模块共享自己保存的地图。现有公共 Cloudflare Worker
使用内置原版地图，连接它时客户端会临时切换为原版地图。
新独立服务端和局域网地图同步要求 **v0.5.0 或更新的客户端**（协议 2）；
v0.5 客户端仍可连接现有 Worker。

## 配置

所有选项均可省略。配置中的路径相对于 `server.json` 所在目录，命令行 `--maps` 的路径
则相对于程序当前工作目录。

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

`room_code` 是八位大写字母或数字，`name` 最多 24 个字符；`max_players` 范围 1–16，
`max_rooms` 范围 1–40，`max_connections_per_ip` 范围 1–256。
把 `allow_room_creation` 设为 `false` 可仅开放配置的小镇，禁止玩家额外开房。
把 `maps_dir` 设为 `null` 可使用内嵌地图。无效或未知配置会提前报错。
命令行支持 `--bind`、`--maps`、`--config`、`--check`、`--init`、`--help`、`--version`；
其中 `--init` 单独使用。

## 可选的服务器密码

在服务端环境变量 **YAPSHIRE_SERVER_PASSWORD** 中设置 8–128 字节的密码，玩家在游戏里
输入框显示为星号的**服务器密码**字段中填写。密码仅在本次游戏进程中保留，不写入设置，
不会随邀请复制，更换地址会清空。请与邀请码分开分享密码。

在 Bash 或 zsh 中，可以隐藏输入并避免把密码写入命令历史：

```sh
read -rs YAPSHIRE_SERVER_PASSWORD
export YAPSHIRE_SERVER_PASSWORD
./yapshire-server --config ./my-town/server.json
```

Docker 可先设置同名环境变量，再给 `docker run` 加上 `--env YAPSHIRE_SERVER_PASSWORD`；
Compose 读取同一变量。Docker 管理员可以查看容器环境变量，因此也要保护宿主机访问权限。
密码同时保护大厅列表和加入连接。`/health` 公开，仅返回状态、版本、协议和地图内容版本。

这是服务器共用密码，不包含玩家账号或管理封禁系统。网络请求使用
`Authorization: Bearer <SHA-256(password)>`，摘要本身也是凭据，不放进 URL。
公网连接请使用 TLS。

## 公网连接与 TLS

在防火墙中放行对应 TCP 端口。家用网络还需要路由器端口转发，运营商 NAT 环境可能需要
VPS 或可互联的 VPN。朋友需要**服务器地址和房间邀请码**，开启密码后还需要密码。
**复制邀请**按钮只复制房间邀请码。

有公网域名时，将服务端绑定到 `127.0.0.1:4761`，交给反向代理提供 TLS。
例如 [Caddy 反向代理](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)可以转发 WebSocket：

```caddyfile
town.example.com {
    reverse_proxy 127.0.0.1:4761
}
```

玩家填写 `wss://town.example.com`。使用有效证书，保留 `/health`、`/rooms`、`/lobby` 和
`/room/*` 路径并转发 Authorization 请求头。代理运行在 Docker 宿主机时，可以将端口
仅映射到本机：`127.0.0.1:4761:4761`。

原生客户端不发送浏览器 Origin；浏览器来源默认拒绝，除非明确加入 `allowed_origins`。
服务端不信任转发的 IP 请求头，按实际 TCP 来源限制流量。反向代理后的玩家会共享代理的
IP 配额：每分钟 300 次连接或大厅请求，以及 `max_connections_per_ip` 连接上限。
规模较大时，需要在代理处实施流量限制，并按这项共享配额规划容量。

每条连接限制客户端消息为 2 KiB、每秒 80 条，发送队列有上限；地图确认超时为 10 秒，
心跳超时为 45 秒。身份由服务端分配，坐标与文本会校验和清理。过慢、畸形或刷消息的连接
会被断开，不影响其他房间。这是客户端驱动移动的社交游戏，不是竞技反作弊方案。
钱包和钓鱼存档仍在玩家自己的电脑，服务端不会保存位置或聊天记录。

## 从源码构建和检查

```sh
cargo build --release --locked -p yapshire-server
cargo test --locked -p yapshire-server -p yapshire-shared
docker build -t yapshire-server:local .
python3 tools/check_container.py yapshire-server:local
```

服务端单独构建不会编译 Bevy，也不需要图形界面依赖。CI 会检查带密码和自定义地图的真实
双客户端连接，并与游戏客户端一起构建四个平台的独立服务端压缩包。
两种容器架构检查通过后才发布 GitHub Release。GHCR 版本镜像标记了源码提交，
Release 压缩包提供 `SHA256SUMS` 校验。
