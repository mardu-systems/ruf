import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { desktopTargets } from "../../scripts/desktop-matrix.ts";

test("PRs build one native target per desktop OS", () => {
  expect(
    desktopTargets(false).map(({ target, runner }) => [target, runner]),
  ).toEqual([
    ["aarch64-apple-darwin", "macos-15"],
    ["x86_64-pc-windows-msvc", "windows-2022"],
    ["x86_64-unknown-linux-gnu", "ubuntu-22.04"],
  ]);
});

test("full runs retain both CPUs, package formats and independent OpenSSL prefixes", () => {
  const full = desktopTargets(true);
  expect(full.map(({ target }) => target)).toEqual([
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
  ]);
  expect(
    full
      .filter(({ target }) => target.includes("linux"))
      .map(({ bundles }) => bundles),
  ).toEqual(["deb,rpm,appimage", "deb,rpm,appimage"]);
  expect(
    full
      .filter(({ target }) => target.includes("apple"))
      .map((row) => row["openssl-prefix"]),
  ).toEqual([
    "native/build/openssl-macos14/install",
    "native/build/openssl-macos14-x86_64/install",
  ]);
  for (const row of desktopTargets(false))
    expect(full.find(({ target }) => target === row.target)).toEqual(row);
});

test("matrix CLI exports usable GitHub output and rejects unknown scopes", () => {
  const root = mkdtempSync(join(tmpdir(), "ruf-desktop-matrix-"));
  const output = join(root, "output");
  const script = resolve(import.meta.dir, "../../scripts/desktop-matrix.ts");
  try {
    const run = (scope: string) =>
      spawnSync(process.execPath, [script, scope], {
        encoding: "utf8",
        env: { ...process.env, GITHUB_OUTPUT: output },
      });
    expect(run("pr").status).toBe(0);
    const saved = readFileSync(output, "utf8");
    expect(JSON.parse(saved.slice("matrix=".length)).include).toEqual(
      desktopTargets(false),
    );
    expect(run("unknown").status).not.toBe(0);
    expect(readFileSync(output, "utf8")).toBe(saved);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
