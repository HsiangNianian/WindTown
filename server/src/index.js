import { DurableObject } from "cloudflare:workers";

// Preserve the address shipped in v0.5.x clients. The Rust server owns all rooms,
// maps and player activity; this Worker only forwards HTTP and WebSocket requests.
export default {
  fetch(request, env) {
    return env.SERVER.fetch(request);
  },
};

// Keep the existing namespaces available for rollback. No new requests reach
// them, and any previously scheduled lobby alarm finishes without rescheduling.
// Removing these exports with a migration would permanently erase their data.
export class Room extends DurableObject {
  fetch() {
    return new Response("Server migrated. Reconnect to join the new lobby.", { status: 410 });
  }

  webSocketMessage(socket) {
    socket.close(1012, "Server migrated. Please reconnect.");
  }

  alarm() {}
}

export class Lobby extends Room {}
