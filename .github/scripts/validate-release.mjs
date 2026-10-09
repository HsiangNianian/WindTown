/** Validate or synchronize workspace/Worker versions without updating dependencies. */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export function validateRelease(root = ".", tag = "", write = false) {
  if (tag) assert.match(tag, /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/, "Use a stable vX.Y.Z tag");
  assert(!write || tag, "--write requires RELEASE_TAG");
  let lock = readFileSync(join(root, "Cargo.lock"), "utf8");
  const manifest = JSON.parse(readFileSync(join(root, "server/package.json"), "utf8"));
  const npmLock = JSON.parse(readFileSync(join(root, "server/package-lock.json"), "utf8"));
  const packages = [["yapshire", "Cargo.toml"], ["yapshire-shared", "crates/yapshire-shared/Cargo.toml"], ["yapshire-server", "crates/yapshire-server/Cargo.toml"]].map(([name, file]) => {
    const cargo = readFileSync(join(root, file), "utf8");
    const source = cargo.match(/\[package\][\s\S]*?\nversion = "([^"]+)"/);
    const locked = lock.match(new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`));
    assert(source && locked, `Missing ${name} package version`);
    return { file, cargo, source, locked };
  });
  const version = tag ? tag.slice(1) : packages[0].source[1];
  assert.match(`v${version}`, /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/);
  if (write) {
    for (const { file, cargo, source, locked } of packages) {
      writeFileSync(join(root, file), cargo.replace(source[0], source[0].replace(`version = "${source[1]}"`, `version = "${version}"`)));
      lock = lock.replace(locked[0], locked[0].replace(`version = "${locked[1]}"`, `version = "${version}"`));
    }
    writeFileSync(join(root, "Cargo.lock"), lock);
    manifest.version = npmLock.version = npmLock.packages[""].version = version;
    for (const [file, value] of [["package.json", manifest], ["package-lock.json", npmLock]]) {
      writeFileSync(join(root, "server", file), JSON.stringify(value, null, 2) + "\n");
    }
  } else {
    for (const actual of [...packages.flatMap(({ source, locked }) => [source[1], locked[1]]), manifest.version, npmLock.version, npmLock.packages[""].version]) {
      assert.equal(actual, version, "Commit matching workspace and Worker versions before tagging");
    }
  }
  return version;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  console.log(validateRelease(".", process.env.RELEASE_TAG || "", process.argv.includes("--write")));
}
