/** Validate platform archives and uploaded checksums before publishing a release draft. */

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { validateRelease } from "./validate-release.mjs";
import { readdirSync, statSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { pathToFileURL } from "node:url";

/** Use the tagged changelog entry verbatim for the release body. */
export function releaseNotes(changelog, tag) {
  const lines = changelog.replaceAll("\r", "").split("\n");
  const start = lines.findIndex((line) => line.startsWith(`## [${tag}] - `));
  assert(start >= 0, `Changelog is missing ${tag}`);
  let end = start + 1;
  while (end < lines.length && !lines[end].startsWith("## ") && !/^\[[^\]]+\]:/.test(lines[end])) end++;
  const notes = lines.slice(start + 1, end).join("\n").trim();
  assert(notes, `Changelog entry for ${tag} is empty`);
  return notes + "\n";
}

/** Require one archive per platform plus the shared changelog and checksum manifest. */
export function collectAssets(root = "release-assets", changelog = "CHANGELOG.md", version = validateRelease()) {
  const platforms = { "linux-x64": ".tar.gz", "windows-x64": ".zip", "macos-arm64": ".tar.gz", "macos-x64": ".tar.gz" };
  const files = [];
  function walk(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile()) files.push(path);
    }
  }
  walk(root);
  const selected = Object.entries(platforms).map(([platform, extension]) => {
    const name = `yapshire-${version}-${platform}${extension}`;
    const matches = files.filter((path) => basename(path) === name);
    assert.equal(matches.length, 1, `Expected one ${name}`);
    return matches[0];
  });
  selected.push(changelog);
  const checksum = join(root, "SHA256SUMS");
  writeFileSync(checksum, selected.map((path) => `${createHash("sha256").update(readFileSync(path)).digest("hex")}  ${basename(path)}`).join("\n") + "\n");
  selected.push(checksum);
  const assets = selected.map((path) => ({ name: basename(path), size: statSync(path).size,
    digest: `sha256:${createHash("sha256").update(readFileSync(path)).digest("hex")}` }));
  assert(assets.every((asset) => asset.size > 0), "Empty release asset");
  assert.equal(new Set(assets.map((asset) => asset.name)).size, assets.length, "Duplicate asset names");
  return assets;
}

/** Publish only a complete draft for the expected tag; leave published releases untouched. */
export async function publishDraft(github, repo, tag, releaseId, expected = JSON.parse(readFileSync("release-assets.json", "utf8"))) {
  assert(Number.isSafeInteger(releaseId) && releaseId > 0, "Missing draft release ID");
  // The tag lookup endpoint does not return unpublished drafts.
  const { data: release } = await github.rest.repos.getRelease({ ...repo, release_id: releaseId });
  assert.equal(release.tag_name, tag, "Release ID does not match the requested tag");
  // Never demote or overwrite an already published release on retry.
  if (!release.draft) return;
  const assets = await github.paginate(github.rest.repos.listReleaseAssets, {
    ...repo, release_id: release.id, per_page: 100,
  });
  for (const wanted of expected) {
    assert(assets.some((asset) => asset.name === wanted.name && asset.size === wanted.size && asset.state === "uploaded" && (!asset.digest || asset.digest === wanted.digest)),
      `Missing or incomplete uploaded asset: ${wanted.name}`);
  }
  await github.rest.repos.updateRelease({
    ...repo, release_id: release.id, draft: false, prerelease: false, make_latest: "legacy",
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.includes("--notes")) {
    const version = validateRelease(".", process.env.RELEASE_TAG || "");
    writeFileSync("release-notes.md", releaseNotes(readFileSync("CHANGELOG.md", "utf8"), `v${version}`));
  } else {
    writeFileSync("release-assets.json", JSON.stringify(collectAssets(), null, 2) + "\n");
  }
}
