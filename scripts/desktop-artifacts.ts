import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";

/** Discover only installers produced by this build, across supported bundles. */
export function desktopInstallers(output: string, started: number): string[] {
  const installers: string[] = [];
  for (const [folder, extension] of [
    ["dmg", ".dmg"],
    ["nsis", ".exe"],
    ["deb", ".deb"],
    ["rpm", ".rpm"],
    ["appimage", ".AppImage"],
  ] as const) {
    const directory = join(output, "bundle", folder);
    let files: string[];
    try {
      files = readdirSync(directory);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") continue;
      throw error;
    }
    for (const file of files) {
      if (!file.endsWith(extension)) continue;
      const path = join(directory, file);
      const info = statSync(path);
      if (info.isFile() && info.mtimeMs >= started) installers.push(path);
    }
  }
  return installers;
}
