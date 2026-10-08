/** Merge one release into the latest default branch, retrying concurrent branch advances. */

import assert from "node:assert/strict";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

/** Insert an absent release in numeric version order and retain existing entries. */
export function mergeChangelog(current, generated, tag) {
  const heading = `## [${tag}] - `;
  if (current.split("\n").some((line) => line.startsWith(heading))) return current;
  const lines = generated.replaceAll("\r", "").split("\n");
  const start = lines.findIndex((line) => line.startsWith(heading));
  assert(start >= 0, `Generated changelog is missing ${tag}`);
  let end = start + 1;
  while (end < lines.length && !lines[end].startsWith("## ") && !/^\[[^\]]+\]:/.test(lines[end])) end++;
  const entry = lines.slice(start, end).join("\n").trimEnd();
  const link = lines.find((line) => line.startsWith(`[${tag}]:`));
  const existing = current.replaceAll("\r", "").trimEnd();
  // Compare numerically so v0.10.0 sorts before v0.2.0.
  const compare = (a, b) => {
    const left = a.slice(1).split(".").map(Number);
    const right = b.slice(1).split(".").map(Number);
    for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] - right[i];
    return 0;
  };
  const next = [...existing.matchAll(/^## \[(v\d+\.\d+\.\d+)\].*$/gm)]
    .find((match) => compare(tag, match[1]) > 0);
  const references = existing.search(/^\[[^\]]+\]:/m);
  const insert = next?.index ?? (references >= 0 ? references : existing.length);
  const header = existing.slice(0, insert).trimEnd();
  const tail = existing.slice(insert);
  return [header || "# Changelog", entry, tail, link].filter(Boolean).join("\n\n") + "\n";
}

/** Commit only the changelog from a clean checkout; fail promptly on permission errors. */
export function commitChangelog({ repo, branch, tag, generated, attempts = 5, beforePush = () => {} }) {
  /** Run Git without shell interpolation and preserve push failures for retry checks. */
  function git(args, allowFailure = false) {
    const result = spawnSync("git", ["-C", repo, ...args], { encoding: "utf8" });
    if (!allowFailure && result.status !== 0) throw new Error(result.stderr || String(result.error));
    return result;
  }
  assert.match(tag, /^v\d+\.\d+\.\d+$/);
  git(["check-ref-format", `refs/heads/${branch}`]);
  assert.equal(git(["status", "--porcelain"]).stdout.trim(), "", "Changelog checkout must be clean");
  for (let attempt = 0; attempt < attempts; attempt++) {
    git(["fetch", "origin", `refs/heads/${branch}`]);
    const base = git(["rev-parse", "FETCH_HEAD"]).stdout.trim();
    git(["checkout", "--detach", base]);
    const path = join(repo, "CHANGELOG.md");
    const current = existsSync(path) ? readFileSync(path, "utf8") : "# Changelog\n";
    const merged = mergeChangelog(current, generated, tag);
    if (merged === current) return;
    writeFileSync(path, merged);
    git(["add", "--", "CHANGELOG.md"]);
    git(["-c", "user.name=github-actions[bot]", "-c", "user.email=41898282+github-actions[bot]@users.noreply.github.com",
      "commit", "-m", `docs: update CHANGELOG.md for ${tag} [skip ci]`]);
    beforePush(attempt);
    const push = git(["push", "origin", `HEAD:refs/heads/${branch}`], true);
    if (push.status === 0) return;
    git(["fetch", "origin", `refs/heads/${branch}`]);
    // Retry only a genuine concurrent branch advance, not auth/protection failures.
    if (git(["rev-parse", "FETCH_HEAD"]).stdout.trim() === base) throw new Error(push.stderr);
  }
  throw new Error(`Changelog push still conflicts after ${attempts} attempts`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  commitChangelog({
    repo: process.env.CHANGELOG_REPO,
    branch: process.env.CHANGELOG_BRANCH,
    tag: process.env.RELEASE_TAG,
    generated: readFileSync("CHANGELOG.md", "utf8"),
  });
}
