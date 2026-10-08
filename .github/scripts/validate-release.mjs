/** Validate or synchronize the committed game/Worker version without updating dependencies. */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export function validateRelease(root = ".", tag = "", write = false) {
  if (tag) assert.match(tag, /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/, "Use a stable vX.Y.Z tag");
  assert(!write || tag, "--write requires RELEASE_TAG");
  const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
  const lock = readFileSync(join(root, "Cargo.lock"), "utf8");
  const manifest = JSON.parse(readFileSync(join(root, "server/package.json"), "utf8"));
  const npmLock = JSON.parse(readFileSync(join(root, "server/package-lock.json"), "utf8"));
  const source = cargo.match(/\[package\][\s\S]*?\nversion = "([^"]+)"/);
  const locked = lock.match(/\[\[package\]\]\nname = "wind-town"\nversion = "([^"]+)"/);
  assert(source && locked, "Missing Wind Town package version");
  const version = tag ? tag.slice(1) : source[1];
  assert.match(`v${version}`, /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/);
  if (write) {
    writeFileSync(join(root, "Cargo.toml"), cargo.replace(source[0], source[0].replace(`version = "${source[1]}"`, `version = "${version}"`)));
    writeFileSync(join(root, "Cargo.lock"), lock.replace(locked[0], locked[0].replace(`version = "${locked[1]}"`, `version = "${version}"`)));
    manifest.version = npmLock.version = npmLock.packages[""].version = version;
    for (const [file, value] of [["package.json", manifest], ["package-lock.json", npmLock]]) {
      writeFileSync(join(root, "server", file), JSON.stringify(value, null, 2) + "\n");
    }
  } else {
    for (const actual of [source[1], locked[1], manifest.version, npmLock.version, npmLock.packages[""].version]) {
      assert.equal(actual, version, "Commit matching Cargo and Worker versions before tagging");
    }
  }
  return version;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  console.log(validateRelease(".", process.env.RELEASE_TAG || "", process.argv.includes("--write")));
}
