# Changelog

Each release entry supplies the corresponding GitHub Release Notes. Entries
prepared before tagging are also bundled in every platform archive. When an
entry is absent, the release workflow generates it from Conventional Commits
and commits it after all platform archives have been uploaded and verified.

## [v0.4.0] - 2026-10-10

### New Features

- Open the in-game map editor from the main menu to customize the town, coast
  and tackle-shop interior across five layers. Paint, erase, fill and pick tiles;
  undo or redo strokes, flip tiles, toggle layers, zoom and pan.
- Use original pixel icons with shortcut badges and hover descriptions for map
  tools, layer visibility and settings. Grid and landmark guides help align edits.
- Save maps locally and apply them immediately. Reopen them in Tiled through
  **Map files**, restore bundled layouts as drafts, and choose to save or discard
  unsaved changes before leaving. Saves keep backups and detect external edits.
- Open the pixel gear **Settings** button from menus, gameplay or the editor to
  switch between English and Simplified Chinese immediately. The game remembers
  the selected language after restarting.
- Translate 239 interface messages through six JSON modules per language.
  Missing, blank or invalid translations fall back to English; named placeholders
  preserve player text. See `docs/TRANSLATING.md` to contribute translations.

### Fixes and Polish

- Keep nicknames, chat drafts and unsaved map edits intact when changing language.
  Block gameplay and editor input while settings are open and prevent the closing
  click from affecting controls behind the panel.
- Validate local map data before loading, preserve invalid files for recovery,
  and release old runtime tile entities when applying a new layout.

### Notes

- Map edits stay on the local computer. Collision, interaction points and NPC
  positions remain fixed, and custom maps are not synchronized between players.
- The multiplayer protocol is unchanged from v0.3.0; existing v0.3.0 Workers
  remain compatible. This release does not require a server redeployment.
- Native macOS checks cover editing, pixel controls, language switching and
  preference recovery after a restart. Windows and macOS builds remain unsigned;
  macOS builds are not notarized.

## [v0.3.0] - 2026-10-09

### New Features

- Visit Tide & Tackle, buy a bamboo rod, hook and bait with a starting wallet of
  100 coins, and sell your catches at Mara's counter.
- Fish at the seaside pier: time the bite, hold Space to reel, and release it to
  manage line tension. Catch sardines, mackerel, sea bass and golden bream.
- Open an illustrated satchel with pixel-art equipment, bait, coins and fish;
  watch the casting line, float, splashes and swimming fish during the minigame.
- Explore a tiled coast and shop interior with animated water. The shipped
  16 × 16 maps can be edited in Tiled without rebuilding the game.
- Save wallet, tackle, bait and catches locally per nickname. LAN and online
  friends can see each other's shop location and fishing pose.

### Fixes and Polish

- Cast into open water beyond the pier and stop at its edge; keep the float
  visible while activity panels are open.
- Align pixel panel borders, improve shop door and counter proportions, place
  the shopkeeper behind the counter, and clear overlapping signs.
- Validate map data and local saves, preserve invalid saves, and provide an
  emergency worm when a player with tackle cannot afford more bait.
- Bundle the map and fishing artwork on every platform and use this changelog
  entry for the release notes and packaged documentation.

### Notes

- Everyone should use v0.3.0 for the updated maps and activities. Self-hosted
  Workers need the matching server update; the built-in public server supports it.
- Progress stays on the local computer; items and money are not shared between
  players. Edited maps retain fixed collision and interaction positions and are
  not synchronized between clients.
- Windows and macOS builds are unsigned; macOS builds are not notarized.

## [v0.2.0] - 2026-10-09
### New Features
- [`c1f9e13`](https://github.com/HsiangNianian/Yapshire/commit/c1f9e1388f9604d80cc15808974bb38295464847) - rename game to Yapshire and add bilingual gameplay previews *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Documentation Changes
- [`036b49d`](https://github.com/HsiangNianian/Yapshire/commit/036b49d72835c30973e022ee70d40dcf20501309) - update CHANGELOG.md for v0.1.0 [skip ci] *(commit by [@github-actions[bot]](https://github.com/apps/github-actions))*

## [v0.1.0] - 2026-10-08
### New Features
- [`3390f15`](https://github.com/HsiangNianian/Yapshire/commit/3390f15c60056c5aa51bc0b7d2f50761586d5573) - add Yapshire multiplayer game and cross-platform releases *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Bug Fixes
- [`6c60c3d`](https://github.com/HsiangNianian/Yapshire/commit/6c60c3d1cc7e01937dc6b77921708a65d81b7ba4) - discover local rooms when broadcast routing is unavailable *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*
- [`9327b22`](https://github.com/HsiangNianian/Yapshire/commit/9327b22fe628fbd230848f2e6a45eccf39d5c7d6) - update Worker tooling to patched dependencies *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*
- [`d8e317b`](https://github.com/HsiangNianian/Yapshire/commit/d8e317b7c53c758d58420b5543cbd86572aa46a1) - keep display modes to a fixed window or fullscreen *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

### Documentation Changes
- [`4efb55f`](https://github.com/HsiangNianian/Yapshire/commit/4efb55f097a1a937cdfa02af952cbacc82b4bb26) - redesign README around game previews and release downloads *(commit by [@HsiangNianian](https://github.com/HsiangNianian))*

[v0.1.0]: https://github.com/HsiangNianian/Yapshire/compare/a21eb24767b62f3b8c9b82bf199d41d1cde36044...v0.1.0

[v0.2.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.1.0...v0.2.0

[v0.3.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.2.0...v0.3.0

[v0.4.0]: https://github.com/HsiangNianian/Yapshire/compare/v0.3.0...v0.4.0
