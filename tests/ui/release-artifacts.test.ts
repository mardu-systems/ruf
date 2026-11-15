import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { collectReleaseArtifacts } from "../../scripts/release-artifacts.ts";

const targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
];
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "ruf-release-"));
  const sha = (value: string) =>
    createHash("sha256").update(value).digest("hex");
  for (const target of targets) {
    const folder = join(root, `desktop-${target}`);
    const linux = target.includes("linux");
    const formats = linux
      ? [
          ["deb", ".deb"],
          ["rpm", ".rpm"],
          ["appimage", ".AppImage"],
        ]
      : [["", target.includes("apple") ? ".dmg" : ".exe"]];
    const installers = formats.map(([directory, extension]) => {
      mkdirSync(join(folder, directory!), { recursive: true });
      const file = `Ruf-${target}${extension}`;
      const bytes = `synthetic ${target} ${extension}`;
      writeFileSync(join(folder, directory!, file), bytes);
      return { file, sha256: sha(bytes) };
    });
    const build = { target, native: true, profile: "release", kind: "desktop" };
    const bill = JSON.stringify({
      metadata: {
        component: { version: "0.1.1" },
        properties: Object.entries(build).map(([key, value]) => ({
          name: `ruf:build:${key}`,
          value: String(value),
        })),
      },
    });
    const sbom = { file: `Ruf-${target}.cdx.json`, sha256: sha(bill) };
    for (const [directory] of formats) {
      writeFileSync(join(folder, directory!, sbom.file), bill);
      writeFileSync(
        join(folder, directory!, `Ruf-${target}.build.json`),
        JSON.stringify({ build, sbom, installers }),
      );
    }
  }
  return root;
}

test("release collection includes both Mac/Linux CPUs and all Linux formats without duplicate sidecars", () => {
  const root = fixture();
  try {
    const files = collectReleaseArtifacts(root, "0.1.1");
    expect(files).toHaveLength(19);
    for (const target of targets)
      expect(files.some((file) => file.includes(target))).toBe(true);
    expect(files.filter((file) => file.endsWith(".rpm"))).toHaveLength(2);
    expect(files.filter((file) => file.endsWith(".cdx.json"))).toHaveLength(5);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

for (const damage of [
  "missing-cpu",
  "wrong-cpu",
  "wrong-version",
  "modified-installer",
  "modified-sbom",
  "different-sidecar",
  "escaping-path",
  "symlink",
  "newline-filename",
] as const) {
  test(`release validation rejects ${damage} before publishing`, () => {
    const root = fixture();
    try {
      const dir = join(root, "desktop-aarch64-unknown-linux-gnu");
      const evidencePath = join(
        dir,
        "deb/Ruf-aarch64-unknown-linux-gnu.build.json",
      );
      const evidence = JSON.parse(readFileSync(evidencePath, "utf8"));
      if (damage === "missing-cpu") rmSync(dir, { recursive: true });
      if (damage === "wrong-cpu") {
        evidence.build.target = "x86_64-unknown-linux-gnu";
        writeFileSync(evidencePath, JSON.stringify(evidence));
      }
      if (damage === "wrong-version")
        expect(() => collectReleaseArtifacts(root, "0.1.2")).toThrow();
      else {
        if (damage === "modified-installer")
          writeFileSync(
            join(dir, "rpm/Ruf-aarch64-unknown-linux-gnu.rpm"),
            "changed",
          );
        if (damage === "modified-sbom")
          writeFileSync(
            join(dir, "deb/Ruf-aarch64-unknown-linux-gnu.cdx.json"),
            "{}",
          );
        if (damage === "different-sidecar")
          writeFileSync(
            join(dir, "appimage/Ruf-aarch64-unknown-linux-gnu.build.json"),
            "{}",
          );
        if (damage === "escaping-path") {
          evidence.sbom.file = "../outside.cdx.json";
          writeFileSync(evidencePath, JSON.stringify(evidence));
        }
        if (damage === "symlink") {
          const installer = join(dir, "rpm/Ruf-aarch64-unknown-linux-gnu.rpm");
          renameSync(installer, `${installer}.original`);
          symlinkSync(`${installer}.original`, installer);
        }
        if (damage === "newline-filename")
          renameSync(evidencePath, `${evidencePath}\n.build.json`);
        expect(() => collectReleaseArtifacts(root, "0.1.1")).toThrow();
      }
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
}
