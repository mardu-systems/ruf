import { expect, test } from "bun:test";
import {
  mkdtempSync,
  mkdirSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { desktopInstallers } from "../../scripts/desktop-artifacts.ts";

test("installer evidence includes fresh Linux packages and excludes stale outputs", () => {
  const root = mkdtempSync(join(tmpdir(), "ruf-installers-"));
  try {
    for (const [folder, file] of [
      ["deb", "Ruf_amd64.deb"],
      ["appimage", "Ruf_aarch64.AppImage"],
      ["rpm", "Ruf.x86_64.rpm"],
      ["nsis", "Ruf-setup.exe"],
      ["dmg", "Ruf.dmg"],
      ["deb", "old.deb"],
      ["deb", "notes.txt"],
    ]) {
      const directory = join(root, "bundle", folder!);
      mkdirSync(directory, { recursive: true });
      writeFileSync(join(directory, file!), "synthetic installer");
    }
    utimesSync(join(root, "bundle/deb/old.deb"), 1, 1);
    expect(
      desktopInstallers(root, Date.now() - 10_000)
        .map((path) => relative(root, path))
        .sort(),
    ).toEqual([
      join("bundle", "appimage", "Ruf_aarch64.AppImage"),
      join("bundle", "deb", "Ruf_amd64.deb"),
      join("bundle", "dmg", "Ruf.dmg"),
      join("bundle", "nsis", "Ruf-setup.exe"),
      join("bundle", "rpm", "Ruf.x86_64.rpm"),
    ]);
    expect(desktopInstallers(join(root, "absent"), 0)).toEqual([]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
