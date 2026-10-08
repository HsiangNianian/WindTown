import { DurableObject } from "cloudflare:workers";

const clean = (value, limit) => [...value.replace(/[\p{Cc}\p{Cf}]/gu, "")].slice(0, limit).join("").trim();
const active = (ctx) => ctx.getWebSockets().filter((ws) => ws.readyState === 1);

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/health") return Response.json({ ok: true, game: "wind-town", protocol: 1 });
    if (request.method === "GET" && (url.pathname === "/rooms" || url.pathname === "/lobby")) {
      const listing = await env.LOBBY.getByName("global").fetch("https://internal/list");
      if (url.pathname === "/rooms") return listing;
      if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") return new Response("WebSocket required", { status: 426 });
      const [client, server] = Object.values(new WebSocketPair());
      server.accept();
      server.send(await listing.text());
      server.close(1000, "Lobby listed");
      return new Response(null, { status: 101, webSocket: client });
    }
    const match = /^\/room\/([A-Z0-9]{8})$/.exec(url.pathname);
    if (!match) return new Response("Wind Town WebSocket server. GET /health", { status: 404 });
    if (request.method !== "GET" || request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
      return new Response("WebSocket upgrade required", { status: 426 });
    }
    if (url.searchParams.get("name")?.length > 128) return new Response("Name too long", { status: 400 });
    if (url.searchParams.get("room_name")?.length > 256) return new Response("Room name too long", { status: 400 });
    return env.ROOMS.getByName(match[1]).fetch(request);
  },
};

export class Room extends DurableObject {
  constructor(ctx, env) {
    super(ctx, env);
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  async fetch(request) {
    const url = new URL(request.url);
    const peers = active(this.ctx);
    if (url.pathname === "/status") return Response.json({ players: peers.length });
    const create = url.searchParams.get("create") === "1";
    if (!create && !peers.length) return new Response("Room not found or already empty", { status: 404 });
    if (create && peers.length) return new Response("Room already exists", { status: 409 });
    if (peers.length >= 16) return new Response("Room is full (16 players)", { status: 409 });
    const name = clean(url.searchParams.get("name") || "Wanderer", 12) || "Wanderer";
    const roomName = clean(url.searchParams.get("room_name") ?? `${name}'s town`, 24);
    if (create && !roomName) return new Response("Room name required", { status: 400 });
    let id;
    do { id = crypto.getRandomValues(new Uint32Array(1))[0]; }
    while (peers.some((ws) => ws.deserializeAttachment().player.id === id));
    const player = { id, name, x: 244 + peers.length * 48, y: 0, moving: false, facing: false };
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    this.ctx.acceptWebSocket(server);
    server.serializeAttachment({ player, second: 0, count: 0, lastChat: 0 });
    if (create) {
      const registration = await this.env.LOBBY.getByName("global").fetch("https://internal/register", {
        method: "POST", body: JSON.stringify({ code: url.pathname.split("/").pop(), name: roomName }),
      });
      if (!registration.ok) { server.close(1013, "Server is full"); return new Response("Server is full", { status: 503 }); }
    }
    server.send(JSON.stringify({ type: "welcome", you: id, players: [...peers.map((ws) => ws.deserializeAttachment().player), player] }));
    this.broadcast({ type: "joined", player }, server);
    return new Response(null, { status: 101, webSocket: client });
  }

  webSocketMessage(ws, raw) {
    if (typeof raw !== "string" || new TextEncoder().encode(raw).length > 2048) {
      ws.close(1009, "Message too large");
      return;
    }
    const state = ws.deserializeAttachment();
    const now = Date.now();
    const second = Math.floor(now / 1000);
    if (state.second !== second) { state.second = second; state.count = 0; }
    if (++state.count > 80) { ws.close(1008, "Too many messages"); return; }
    let msg;
    try { msg = JSON.parse(raw); }
    catch { ws.close(1007, "Invalid JSON"); return; }
    if (!msg || typeof msg !== "object") { ws.close(1007, "Invalid message"); return; }
    if (msg.type === "move") {
      if (!Number.isFinite(msg.x) || !Number.isFinite(msg.y) || typeof msg.moving !== "boolean" || typeof msg.facing !== "boolean") {
        ws.close(1007, "Invalid movement"); return;
      }
      // ponytail: positions are client-driven for this social toy; use authoritative physics for competitive play.
      Object.assign(state.player, { x: Math.max(12, Math.min(1428, msg.x)), y: Math.max(0, Math.min(96, msg.y)), moving: msg.moving, facing: msg.facing });
      this.broadcast({ type: "moved", player: state.player }, ws);
    } else if (msg.type === "chat" && typeof msg.text === "string") {
      const text = clean(msg.text, 80);
      if (text && now - state.lastChat >= 300) {
        state.lastChat = now;
        this.broadcast({ type: "chat", id: state.player.id, text });
      }
    } else { ws.close(1007, "Unknown message"); return; }
    ws.serializeAttachment(state);
  }

  broadcast(msg, except) {
    const data = JSON.stringify(msg);
    for (const peer of active(this.ctx)) {
      if (peer === except) continue;
      try { peer.send(data); }
      catch { peer.close(1011, "Connection lost"); }
    }
  }

  webSocketClose(ws, code) {
    const state = ws.deserializeAttachment();
    ws.close(code === 1005 || code === 1006 ? 1000 : code, "Left room");
    if (state) this.broadcast({ type: "left", id: state.player.id }, ws);
  }

  webSocketError(ws) {
    const state = ws.deserializeAttachment();
    ws.close(1011, "Connection lost");
    if (state) this.broadcast({ type: "left", id: state.player.id }, ws);
  }
}

export class Lobby extends DurableObject {
  async fetch(request) {
    if (request.method === "POST") {
      const { code, name } = await request.json();
      if (!/^[A-Z0-9]{8}$/.test(code) || typeof name !== "string") return new Response("Invalid room", { status: 400 });
      const records = await this.ctx.storage.list({ prefix: "room:", limit: 40 });
      if (records.size >= 40) return new Response("Lobby full", { status: 503 });
      await this.ctx.storage.put(`room:${code}`, { code, name: clean(name, 24), created: Date.now() });
      await this.ctx.storage.setAlarm(Date.now() + 60000);
      return new Response("Registered");
    }
    const rooms = await this.liveRooms();
    return Response.json({ rooms }, { headers: { "Cache-Control": "no-store" } });
  }

  async liveRooms() {
    const records = await this.ctx.storage.list({ prefix: "room:", limit: 40 });
    const rooms = await Promise.all([...records].map(async ([key, room]) => {
      const response = await this.env.ROOMS.getByName(room.code).fetch("https://internal/status");
      const { players } = await response.json();
      if (!players) { await this.ctx.storage.delete(key); return null; }
      return { code: room.code, name: room.name, players };
    }));
    return rooms.filter(Boolean).sort((a, b) => a.code.localeCompare(b.code));
  }

  async alarm() {
    if ((await this.liveRooms()).length) await this.ctx.storage.setAlarm(Date.now() + 60000);
  }
}
