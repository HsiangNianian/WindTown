use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, protocol::WebSocketConfig, stream::MaybeTlsStream};
use url::Url;

pub const PORT: u16 = 4761;
pub const DEFAULT_SERVER: &str = include_str!("../assets/server-url.txt");
const DISCOVERY_PORT: u16 = 4762;
const DISCOVER: &[u8] = b"WIND_TOWN_DISCOVER_V1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEntry {
    #[serde(default)]
    pub code: String,
    pub name: String,
    pub players: usize,
    #[serde(default)]
    pub address: String,
}

#[derive(Serialize, Deserialize)]
struct LanAnnouncement {
    id: u64,
    name: String,
    players: usize,
    port: u16,
}

pub fn discover_lan() -> Result<Vec<RoomEntry>, String> {
    scan_lan(DISCOVERY_PORT)
}

fn scan_lan(port: u16) -> Result<Vec<RoomEntry>, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    socket
        .send_to(DISCOVER, ("255.255.255.255", port))
        .map_err(|e| e.to_string())?;
    let _ = socket.send_to(DISCOVER, ("127.0.0.1", port));
    let start = Instant::now();
    let mut rooms = HashMap::<u64, RoomEntry>::new();
    let mut buffer = [0; 512];
    while start.elapsed() < Duration::from_millis(1400) {
        if let Ok((len, source)) = socket.recv_from(&mut buffer) {
            if let Ok(reply) = serde_json::from_slice::<LanAnnouncement>(&buffer[..len]) {
                if reply.port == 0 || reply.players > 16 {
                    continue;
                }
                let address = format!("{}:{}", source.ip(), reply.port);
                if rooms.len() < 64
                    && (!source.ip().is_loopback() || !rooms.contains_key(&reply.id))
                {
                    rooms.insert(
                        reply.id,
                        RoomEntry {
                            code: String::new(),
                            name: clean(&reply.name, 12),
                            players: reply.players,
                            address,
                        },
                    );
                }
            }
        }
    }
    let mut rooms: Vec<_> = rooms.into_values().collect();
    rooms.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(rooms)
}

pub fn discover_cloud(address: &str) -> Result<Vec<RoomEntry>, String> {
    let mut url = url_for(address, "", "", false)?;
    url.set_path("/lobby");
    url.set_query(None);
    let mut socket = open_socket(&url, &AtomicBool::new(false))?;
    #[derive(Deserialize)]
    struct Listing {
        rooms: Vec<RoomEntry>,
    }
    let message = socket.read().map_err(|e| e.to_string())?;
    let listing: Listing = serde_json::from_str(message.to_text().map_err(|e| e.to_string())?)
        .map_err(|_| "Server does not support the lobby".to_owned())?;
    let _ = socket.close(None);
    Ok(listing
        .rooms
        .into_iter()
        .filter(|r| {
            r.code.len() == 8
                && r.code.bytes().all(|c| c.is_ascii_alphanumeric())
                && r.players <= 16
        })
        .take(100)
        .map(|mut r| {
            r.name = clean(&r.name, 24);
            r
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub moving: bool,
    pub facing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Move {
        x: f32,
        y: f32,
        moving: bool,
        facing: bool,
    },
    Chat {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome { you: u32, players: Vec<Player> },
    Joined { player: Player },
    Moved { player: Player },
    Chat { id: u32, text: String },
    Left { id: u32 },
}

pub enum Mode {
    HostLan(u16),
    JoinLan(String),
    HostCloud { server: String, room_name: String },
    JoinCloud { server: String, room: String },
}

pub enum Event {
    Room { label: String, invite: String },
    Message(ServerMessage),
    Error(String),
}

pub struct Link {
    pub send: SyncSender<ClientMessage>,
    pub events: Mutex<Receiver<Event>>,
    stop: Arc<AtomicBool>,
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn clean(text: &str, limit: usize) -> String {
    text.chars().filter(|c| !c.is_control() && !matches!(*c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}')).take(limit).collect::<String>().trim().to_owned()
}

pub fn start(mode: Mode, name: String) -> Link {
    let (send, outgoing) = mpsc::sync_channel(64);
    let (incoming, events) = mpsc::sync_channel(256);
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    thread::spawn(move || {
        if let Err(err) = client(mode, &name, outgoing, &incoming, thread_stop.clone()) {
            let _ = incoming.try_send(Event::Error(err));
        }
        thread_stop.store(true, Ordering::Relaxed);
    });
    Link {
        send,
        events: Mutex::new(events),
        stop,
    }
}

fn url_for(address: &str, room: &str, name: &str, create: bool) -> Result<Url, String> {
    let address = address.trim();
    let address = if address.contains("://") {
        address.to_owned()
    } else {
        format!("ws://{address}")
    };
    let mut url = Url::parse(&address).map_err(|_| "Invalid server address".to_owned())?;
    let scheme = match url.scheme() {
        "https" | "wss" => "wss",
        "http" | "ws" => "ws",
        _ => return Err("Use a ws:// or wss:// address".into()),
    };
    url.set_scheme(scheme).map_err(|_| "Invalid address")?;
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Enter a server address without credentials".into());
    }
    url.set_path(&format!("/room/{room}"));
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut()
        .append_pair("name", name)
        .append_pair("create", if create { "1" } else { "0" });
    Ok(url)
}

fn config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(16 * 1024))
        .max_frame_size(Some(16 * 1024))
}

fn client(
    mode: Mode,
    name: &str,
    outgoing: Receiver<ClientMessage>,
    events: &SyncSender<Event>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let name = clean(name, 12);
    let name = if name.is_empty() { "Wanderer" } else { &name };
    let (url, label, invite) = match mode {
        Mode::HostLan(port) => {
            let listener = TcpListener::bind(("0.0.0.0", port))
                .map_err(|e| format!("Cannot host on port {port}: {e}"))?;
            let port = listener.local_addr().map_err(|e| e.to_string())?.port();
            // ponytail: one LAN host per computer; share discovery before supporting multiple hosts.
            let discovery = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)).map_err(|e| {
                format!(
                    "LAN discovery port {DISCOVERY_PORT} is busy. Close the other local host: {e}"
                )
            })?;
            serve(listener, discovery, stop.clone(), name.to_owned()).map_err(|e| e.to_string())?;
            let invite = format!("{}:{port}", lan_ip());
            (
                url_for(&format!("127.0.0.1:{port}"), "LOCAL", name, false)?,
                format!("LAN HOST / {invite}"),
                invite,
            )
        }
        Mode::JoinLan(address) => (
            url_for(&address, "LOCAL", name, false)?,
            format!("LAN / {address}"),
            address,
        ),
        Mode::HostCloud { server, room_name } => {
            let room_name = clean(&room_name, 24);
            if room_name.is_empty() {
                return Err("Please enter a room name.".into());
            }
            let room: String = rand::rng()
                .sample_iter(Alphanumeric)
                .take(8)
                .map(char::from)
                .collect::<String>()
                .to_uppercase();
            let mut url = url_for(&server, &room, name, true)?;
            url.query_pairs_mut().append_pair("room_name", &room_name);
            (url, format!("{room_name} / {room}"), room)
        }
        Mode::JoinCloud { server, room } => {
            let room = room.trim().to_uppercase();
            if room.len() != 8 || !room.bytes().all(|c| c.is_ascii_alphanumeric()) {
                return Err("Enter an 8-character room code".into());
            }
            (
                url_for(&server, &room, name, false)?,
                format!("ROOM / {room}"),
                room,
            )
        }
    };
    let mut socket = open_socket(&url, &stop)?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(Duration::from_millis(10))),
        MaybeTlsStream::NativeTls(s) => s
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(10))),
        _ => Err(io::Error::other("Unsupported connection type")),
    }
    .map_err(|e| e.to_string())?;
    events
        .try_send(Event::Room { label, invite })
        .map_err(|e| e.to_string())?;
    let mut heartbeat = Instant::now();
    let mut last_received = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        for msg in outgoing.try_iter() {
            socket
                .send(Message::text(
                    serde_json::to_string(&msg).map_err(|e| e.to_string())?,
                ))
                .map_err(|e| e.to_string())?;
        }
        if heartbeat.elapsed() >= Duration::from_secs(15) {
            socket
                .send(Message::text("ping"))
                .map_err(|e| e.to_string())?;
            heartbeat = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                last_received = Instant::now();
                if text != "pong" {
                    let msg = serde_json::from_str(&text)
                        .map_err(|_| "Server protocol mismatch".to_owned())?;
                    events
                        .try_send(Event::Message(msg))
                        .map_err(|_| "Network queue full. Please reconnect.".to_owned())?;
                }
            }
            Ok(Message::Close(_)) => {
                return Err("Connection closed. Press Esc to return and rejoin.".into());
            }
            Ok(_) => {
                last_received = Instant::now();
            }
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(format!("Disconnected: {e}")),
        }
        if last_received.elapsed() > Duration::from_secs(45) {
            return Err("Server timed out. Press Esc to return and rejoin.".into());
        }
    }
    let _ = socket.close(None);
    Ok(())
}

fn open_socket(
    url: &Url,
    stop: &AtomicBool,
) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
    // Retry only before the WebSocket request: retrying a room creation could duplicate it.
    let mut stream = open_transport(url, stop);
    for _ in 0..2 {
        if stream.is_ok() || stop.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(250));
        stream = open_transport(url, stop);
    }
    let (socket, _) =
        tungstenite::client::client_with_config(url.as_str(), stream?, Some(config())).map_err(
            |e| {
                let detail = e.to_string();
                if detail.contains("404") {
                    "Room not found, or everyone has left.".into()
                } else if detail.contains("409") {
                    "Room is full or the code is taken. Try again.".into()
                } else {
                    format!("WebSocket connection failed: {detail}")
                }
            },
        )?;
    Ok(socket)
}

fn open_transport(url: &Url, stop: &AtomicBool) -> Result<MaybeTlsStream<TcpStream>, String> {
    let host = url.host_str().ok_or("Missing hostname")?;
    let port = url.port_or_known_default().ok_or("Missing port")?;
    let proxy = if url.scheme() == "wss" && !bypass_proxy(host) {
        ["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
            .map(|value| Url::parse(&value).map_err(|_| "Invalid HTTPS proxy configuration"))
            .transpose()?
    } else {
        None
    };
    if let Some(proxy) = &proxy {
        if proxy.scheme() != "http" || !proxy.username().is_empty() || proxy.password().is_some() {
            return Err(
                "This client supports unauthenticated HTTP CONNECT proxies. Check HTTPS_PROXY."
                    .into(),
            );
        }
    }
    let endpoint = proxy.as_ref().map_or((host, port), |p| {
        (
            p.host_str().unwrap_or("localhost"),
            p.port_or_known_default().unwrap_or(80),
        )
    });
    let addresses: Vec<SocketAddr> = endpoint
        .to_socket_addrs()
        .map_err(|e| format!("Cannot resolve server: {e}"))?
        .collect();
    let mut stream = None;
    for address in addresses {
        if stop.load(Ordering::Relaxed) {
            return Err("Connection cancelled".into());
        }
        if let Ok(s) = TcpStream::connect_timeout(&address, Duration::from_secs(4)) {
            stream = Some(s);
            break;
        }
    }
    let mut stream = stream.ok_or("Cannot connect. Check the address, host and firewall.")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(12)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    if proxy.is_some() {
        write!(
            stream,
            "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n"
        )
        .map_err(|e| e.to_string())?;
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            if headers.len() >= 8192 {
                return Err("Proxy response is too large".into());
            }
            let mut byte = [0];
            stream
                .read_exact(&mut byte)
                .map_err(|e| format!("Proxy failed: {e}"))?;
            headers.push(byte[0]);
        }
        if !String::from_utf8_lossy(&headers)
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .is_some_and(|s| s == "200")
        {
            return Err("HTTPS proxy rejected the connection".into());
        }
    }
    // Handle timed-out TLS handshakes here; tungstenite's native-TLS helper panics on WouldBlock.
    let stream = if url.scheme() == "wss" {
        let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        MaybeTlsStream::NativeTls(
            tls.connect(host, stream)
                .map_err(|e| format!("TLS connection failed; try refreshing: {e}"))?,
        )
    } else {
        MaybeTlsStream::Plain(stream)
    };
    Ok(stream)
}

fn bypass_proxy(host: &str) -> bool {
    if host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return true;
    }
    std::env::var("no_proxy")
        .or_else(|_| std::env::var("NO_PROXY"))
        .unwrap_or_default()
        .split(',')
        .any(|entry| {
            let entry = entry.trim().trim_start_matches('.');
            !entry.is_empty()
                && (entry == "*" || host == entry || host.ends_with(&format!(".{entry}")))
        })
}

pub fn lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("192.0.2.1:9")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

struct Peer {
    player: Player,
    send: SyncSender<ServerMessage>,
}
type Peers = Arc<Mutex<HashMap<u32, Peer>>>;

fn broadcast(peers: &HashMap<u32, Peer>, msg: ServerMessage, except: Option<u32>) {
    for (&id, peer) in peers {
        if Some(id) != except {
            let _ = peer.send.try_send(msg.clone());
        }
    }
}

pub fn serve(
    listener: TcpListener,
    discovery: UdpSocket,
    stop: Arc<AtomicBool>,
    owner: String,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    discovery.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let instance = rand::random::<u64>();
    thread::spawn(move || {
        let peers: Peers = Arc::default();
        let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        while !stop.load(Ordering::Relaxed) {
            let mut buffer = [0; 64];
            for _ in 0..16 {
                let Ok((len, source)) = discovery.recv_from(&mut buffer) else {
                    break;
                };
                if &buffer[..len] == DISCOVER {
                    let room = LanAnnouncement {
                        id: instance,
                        name: owner.clone(),
                        players: peers.lock().unwrap().len(),
                        port,
                    };
                    if let Ok(bytes) = serde_json::to_vec(&room) {
                        let _ = discovery.send_to(&bytes, source);
                    }
                }
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    if pending.load(Ordering::Relaxed) >= 16 {
                        continue;
                    }
                    pending.fetch_add(1, Ordering::Relaxed);
                    let (peers, stop, pending) = (peers.clone(), stop.clone(), pending.clone());
                    thread::spawn(move || {
                        let _ = serve_peer(stream, peers, stop);
                        pending.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
    });
    Ok(())
}

fn serve_peer(
    stream: TcpStream,
    peers: Peers,
    stop: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    stream.set_nodelay(true)?;
    let mut name = "Wanderer".to_owned();
    let mut socket = tungstenite::accept_hdr_with_config(
        stream,
        |req: &tungstenite::handshake::server::Request, response| {
            if req.uri().path() != "/room/LOCAL" {
                return Err(tungstenite::http::Response::builder()
                    .status(404)
                    .body(Some("Unknown room".into()))
                    .unwrap());
            }
            if let Some(query) = req.uri().query() {
                if let Some((_, value)) =
                    url::form_urlencoded::parse(query.as_bytes()).find(|(k, _)| k == "name")
                {
                    name = clean(&value, 12);
                    if name.is_empty() {
                        name = "Wanderer".into();
                    }
                }
            }
            Ok(response)
        },
        Some(config()),
    )
    .map_err(|e| e.to_string())?;
    socket
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(10)))?;
    let (tx, rx) = mpsc::sync_channel(128);
    let id;
    {
        let mut peers = peers.lock().unwrap();
        if peers.len() >= 16 {
            socket.close(None)?;
            return Ok(());
        }
        id = loop {
            let id = rand::random();
            if !peers.contains_key(&id) {
                break id;
            }
        };
        let player = Player {
            id,
            name,
            x: 244.0 + peers.len() as f32 * 48.0,
            y: 0.0,
            moving: false,
            facing: false,
        };
        broadcast(
            &peers,
            ServerMessage::Joined {
                player: player.clone(),
            },
            None,
        );
        peers.insert(
            id,
            Peer {
                player,
                send: tx.clone(),
            },
        );
        tx.try_send(ServerMessage::Welcome {
            you: id,
            players: peers.values().map(|p| p.player.clone()).collect(),
        })?;
    }
    let result = relay(&mut socket, &peers, id, &rx, &stop);
    {
        let mut peers = peers.lock().unwrap();
        peers.remove(&id);
        broadcast(&peers, ServerMessage::Left { id }, None);
    }
    let _ = socket.close(None);
    result
}

fn relay(
    socket: &mut WebSocket<TcpStream>,
    peers: &Peers,
    id: u32,
    rx: &Receiver<ServerMessage>,
    stop: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut last_chat = Instant::now() - Duration::from_secs(1);
    let mut last_seen = Instant::now();
    let mut rate = (Instant::now(), 0);
    while !stop.load(Ordering::Relaxed) {
        for msg in rx.try_iter() {
            socket.send(Message::text(serde_json::to_string(&msg)?))?;
        }
        let raw = match socket.read() {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                if last_seen.elapsed() > Duration::from_secs(45) {
                    break;
                }
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        last_seen = Instant::now();
        if rate.0.elapsed() >= Duration::from_secs(1) {
            rate = (Instant::now(), 0);
        }
        rate.1 += 1;
        if rate.1 > 80 || raw.len() > 2048 {
            break;
        }
        if raw == "ping" {
            socket.send(Message::text("pong"))?;
            continue;
        }
        let message: ClientMessage = serde_json::from_str(&raw)?;
        let mut peers = peers.lock().unwrap();
        match message {
            ClientMessage::Move {
                x,
                y,
                moving,
                facing,
            } => {
                if !x.is_finite() || !y.is_finite() {
                    break;
                }
                let Some(peer) = peers.get_mut(&id) else {
                    break;
                };
                // ponytail: client-driven, bounded positions; add authoritative physics for competitive play.
                peer.player.x = x.clamp(12.0, 1428.0);
                peer.player.y = y.clamp(0.0, 96.0);
                peer.player.moving = moving;
                peer.player.facing = facing;
                let player = peer.player.clone();
                broadcast(&peers, ServerMessage::Moved { player }, Some(id));
            }
            ClientMessage::Chat { text } => {
                let text = clean(&text, 80);
                if !text.is_empty() && last_chat.elapsed() >= Duration::from_millis(300) {
                    last_chat = Instant::now();
                    broadcast(&peers, ServerMessage::Chat { id, text }, None);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalled_tls_returns_an_error_instead_of_panicking() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (release, hold) = mpsc::channel::<()>();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            let _ = hold.recv_timeout(Duration::from_secs(20));
        });
        let url = Url::parse(&format!("wss://{address}/room/ABCDEFGH")).unwrap();
        let result = open_transport(&url, &AtomicBool::new(false));
        let _ = release.send(());
        server.join().unwrap();
        assert!(matches!(result, Err(error) if error.contains("TLS connection failed")));
    }

    #[test]
    fn broken_tls_is_retried_at_most_three_times() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().unwrap();
                drop(stream);
            }
        });
        let url = Url::parse(&format!("wss://{address}/room/ABCDEFGH")).unwrap();
        assert!(open_socket(&url, &AtomicBool::new(false)).is_err());
        server.join().unwrap();
    }

    #[test]
    #[ignore = "requires WIND_TOWN_TEST_SERVER pointing to a running Worker"]
    fn cloud_client_hosts_discovers_joins_moves_and_chats() {
        let server = std::env::var("WIND_TOWN_TEST_SERVER").unwrap();
        fn welcome(link: &Link) -> (u32, String) {
            let mut code = String::new();
            loop {
                match link
                    .events
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(15))
                    .unwrap()
                {
                    Event::Room { invite, .. } => code = invite,
                    Event::Message(ServerMessage::Welcome { you, .. }) => return (you, code),
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
        }
        fn next(link: &Link, kind: &str) -> ServerMessage {
            let deadline = Instant::now() + Duration::from_secs(15);
            while Instant::now() < deadline {
                match link
                    .events
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                {
                    Event::Message(msg)
                        if matches!(
                            (&msg, kind),
                            (ServerMessage::Moved { .. }, "move")
                                | (ServerMessage::Chat { .. }, "chat")
                                | (ServerMessage::Left { .. }, "left")
                        ) =>
                    {
                        return msg;
                    }
                    Event::Error(error) => panic!("{error}"),
                    _ => {}
                }
            }
            panic!("Missing {kind}");
        }
        let a = start(
            Mode::HostCloud {
                server: server.clone(),
                room_name: "A quiet afternoon".into(),
            },
            "Rust Host".into(),
        );
        let (id, code) = welcome(&a);
        let listing = discover_cloud(&server).unwrap();
        assert!(
            listing
                .iter()
                .any(|r| r.code == code && r.players == 1 && r.name == "A quiet afternoon")
        );
        let b = start(
            Mode::JoinCloud {
                server: server.clone(),
                room: code,
            },
            "Rust Guest".into(),
        );
        let (other, _) = welcome(&b);
        assert_ne!(id, other);
        a.send
            .send(ClientMessage::Move {
                x: 600.0,
                y: 20.0,
                moving: true,
                facing: false,
            })
            .unwrap();
        assert!(
            matches!(next(&b, "move"), ServerMessage::Moved { player } if player.id == id && player.x == 600.0)
        );
        a.send
            .send(ClientMessage::Chat {
                text: "Hello from Rust!".into(),
            })
            .unwrap();
        assert!(
            matches!(next(&b, "chat"), ServerMessage::Chat { id: speaker, text } if speaker == id && text == "Hello from Rust!")
        );
        drop(a);
        assert!(matches!(next(&b, "left"), ServerMessage::Left { id: left } if left == id));
    }

    #[test]
    fn text_and_addresses_are_bounded() {
        assert_eq!(clean("\n 你好\u{202e}\0 world \t", 5), "你好 w");
        assert_eq!(clean(&"中".repeat(200), 80).chars().count(), 80);
        let url = url_for("https://example.com/", "ABCDEFGH", "小风&雨", true).unwrap();
        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.path(), "/room/ABCDEFGH");
        assert!(url.as_str().contains("%26"));
        assert!(url_for("file:///etc/passwd", "ABCD", "x", false).is_err());
    }

    #[test]
    fn two_real_websockets_sync_move_chat_and_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let discovery = UdpSocket::bind("0.0.0.0:0").unwrap();
        let discovery_port = discovery.local_addr().unwrap().port();
        serve(listener, discovery, stop.clone(), "Test Host".into()).unwrap();
        assert!(
            scan_lan(discovery_port)
                .unwrap()
                .iter()
                .any(|r| r.name == "Test Host" && r.address.ends_with(&addr.port().to_string()))
        );
        let connect = |name| {
            let url = url_for(&addr.to_string(), "LOCAL", name, false).unwrap();
            let (mut ws, _) = tungstenite::connect(url.as_str()).unwrap();
            if let MaybeTlsStream::Plain(s) = ws.get_mut() {
                s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            }
            ws
        };
        let read = |ws: &mut WebSocket<MaybeTlsStream<TcpStream>>| -> ServerMessage {
            serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap()
        };
        let mut a = connect("小风");
        let ServerMessage::Welcome { you, .. } = read(&mut a) else {
            panic!("welcome")
        };
        let mut b = connect("小雨");
        assert!(
            matches!(read(&mut b), ServerMessage::Welcome { players, .. } if players.len() == 2)
        );
        assert!(matches!(read(&mut a), ServerMessage::Joined { .. }));
        a.send(Message::text(
            r#"{"type":"move","x":700,"y":12,"moving":true,"facing":true}"#,
        ))
        .unwrap();
        assert!(
            matches!(read(&mut b), ServerMessage::Moved { player } if player.id == you && player.x == 700.0 && player.facing)
        );
        a.send(Message::text(r#"{"type":"chat","text":"你好，小镇！"}"#))
            .unwrap();
        assert!(matches!(read(&mut a), ServerMessage::Chat { text, .. } if text == "你好，小镇！"));
        assert!(matches!(read(&mut b), ServerMessage::Chat { id, .. } if id == you));
        a.close(None).unwrap();
        assert!(matches!(read(&mut b), ServerMessage::Left { id } if id == you));
        stop.store(true, Ordering::Relaxed);
    }
}
