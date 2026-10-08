/** Exercise version synchronization, archive verification and retry-safe changelog publishing. */
import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, cpSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { validateRelease } from "./validate-release.mjs";
import { mergeChangelog, commitChangelog } from "./commit-changelog.mjs";
import { collectAssets, publishDraft } from "./release-assets.mjs";

const entry = (tag) => `## [${tag}] - 2026-10-08\n\n### Features\n\n- Meet friends.\n\n[${tag}]: https://example.com/releases/${tag}\n`;

test("version sync changes only project versions and rejects mismatched tags", () => {
  const root = mkdtempSync(join(tmpdir(), "wind-town-version-"));
  try {
    mkdirSync(join(root, "server"));
    for (const name of ["Cargo.toml", "Cargo.lock", "server/package.json", "server/package-lock.json"]) cpSync(resolve(name), join(root, name));
    const dependencies = JSON.parse(readFileSync(join(root, "server/package-lock.json"))).packages;
    const current = validateRelease(root);
    assert.throws(() => validateRelease(root, "v9.8.7"));
    for (const invalid of ["v1.2.3-beta", "v01.2.3", "1.2.3", "v1.2.3;echo bad"]) assert.throws(() => validateRelease(root, invalid));
    assert.equal(validateRelease(root, "v9.8.7", true), "9.8.7");
    assert.equal(validateRelease(root, "v9.8.7"), "9.8.7");
    const after = JSON.parse(readFileSync(join(root, "server/package-lock.json"))).packages;
    delete dependencies[""]; delete after[""];
    assert.deepEqual(after, dependencies);
    validateRelease(root, `v${current}`, true);
    assert.equal(readFileSync(join(root, "Cargo.lock"), "utf8"), readFileSync("Cargo.lock", "utf8"));
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("changelog merges numerically, preserves existing entries and is idempotent", () => {
  const current = `# Changelog\n\n${entry("v0.2.0")}`;
  const merged = mergeChangelog(current, entry("v0.10.0"), "v0.10.0");
  assert(merged.indexOf("## [v0.10.0]") < merged.indexOf("## [v0.2.0]"));
  assert.equal(mergeChangelog(merged, entry("v0.10.0"), "v0.10.0"), merged);
  assert(mergeChangelog(merged, entry("v0.1.0"), "v0.1.0").includes("## [v0.1.0]"));
  assert.throws(() => mergeChangelog(current, "missing", "v1.0.0"));
});

test("release requires all four archives and a complete uploaded draft", async () => {
  const root = mkdtempSync(join(tmpdir(), "wind-town-assets-"));
  try {
    const changelog = join(root, "CHANGELOG.md");
    writeFileSync(changelog, entry("v1.2.3"));
    for (const [platform, ext] of [["linux-x64", "tar.gz"], ["windows-x64", "zip"], ["macos-arm64", "tar.gz"], ["macos-x64", "tar.gz"]]) {
      writeFileSync(join(root, `wind-town-1.2.3-${platform}.${ext}`), `archive for ${platform}`);
    }
    const expected = collectAssets(root, changelog, "1.2.3");
    assert.equal(expected.length, 6);
    assert(readFileSync(join(root, "SHA256SUMS"), "utf8").includes("wind-town-1.2.3-windows-x64.zip"));
    let uploaded = expected.map((asset) => ({ ...asset, state: "uploaded" }));
    let published = false;
    const github = { paginate: async () => uploaded, rest: { repos: {
      getRelease: async () => ({ data: { id: 42, tag_name: "v1.2.3", draft: true } }),
      listReleaseAssets: () => {},
      updateRelease: async () => { published = true; },
    } } };
    uploaded = uploaded.slice(1);
    await assert.rejects(publishDraft(github, {}, "v1.2.3", 42, expected));
    assert.equal(published, false);
    uploaded = expected.map((asset) => ({ ...asset, state: "uploaded" }));
    uploaded[0].digest = "sha256:wrong";
    await assert.rejects(publishDraft(github, {}, "v1.2.3", 42, expected));
    uploaded[0].digest = expected[0].digest;
    await publishDraft(github, {}, "v1.2.3", 42, expected);
    assert.equal(published, true);
    published = false;
    github.rest.repos.getRelease = async () => ({ data: { id: 42, tag_name: "v1.2.3", draft: false } });
    await publishDraft(github, {}, "v1.2.3", 42, expected);
    assert.equal(published, false);
    rmSync(join(root, "wind-town-1.2.3-linux-x64.tar.gz"));
    assert.throws(() => collectAssets(root, changelog, "1.2.3"));
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("a concurrent default-branch commit survives changelog writeback and retry", () => {
  const root = mkdtempSync(join(tmpdir(), "wind-town-git-"));
  const git = (cwd, ...args) => execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  try {
    const remote = join(root, "remote.git"), work = join(root, "work"), other = join(root, "other");
    git(root, "init", "--bare", "--initial-branch=main", remote);
    git(root, "clone", remote, work);
    git(work, "config", "user.name", "Test"); git(work, "config", "user.email", "test@example.com");
    writeFileSync(join(work, "CHANGELOG.md"), "# Changelog\n");
    git(work, "add", "."); git(work, "commit", "-m", "chore: initialize"); git(work, "push", "origin", "main");
    git(root, "clone", remote, other);
    git(other, "config", "user.name", "Test"); git(other, "config", "user.email", "test@example.com");
    commitChangelog({ repo: work, branch: "main", tag: "v1.0.0", generated: entry("v1.0.0"), beforePush: (attempt) => {
      if (attempt) return;
      writeFileSync(join(other, "keep.txt"), "user change");
      git(other, "add", "."); git(other, "commit", "-m", "docs: keep my change"); git(other, "push", "origin", "main");
    } });
    assert.equal(readFileSync(join(work, "keep.txt"), "utf8"), "user change");
    const head = git(work, "rev-parse", "HEAD");
    commitChangelog({ repo: work, branch: "main", tag: "v1.0.0", generated: entry("v1.0.0") });
    assert.equal(git(work, "rev-parse", "HEAD"), head);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
