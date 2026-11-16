// Keep package/cache metadata in one place for the PR and full desktop matrices.
import { appendFileSync } from "node:fs";

interface DesktopTarget {
  runner: string;
  target: string;
  bundle: string;
  bundles: string;
  artifact: string;
  "openssl-cache-path": string;
  "openssl-recipe": string;
  "openssl-prefix"?: string;
  "extra-evidence"?: string;
}

const targets: DesktopTarget[] = [
  {
    runner: "macos-15",
    target: "aarch64-apple-darwin",
    bundle: "dmg",
    bundles: "app,dmg",
    artifact: "target/aarch64-apple-darwin/release/bundle/dmg/*.dmg",
    "openssl-cache-path": "native/build/openssl-macos14/install",
    "openssl-prefix": "native/build/openssl-macos14/install",
    "openssl-recipe": "native/build-openssl-macos.sh",
  },
  {
    runner: "macos-15-intel",
    target: "x86_64-apple-darwin",
    bundle: "dmg",
    bundles: "app,dmg",
    artifact: "target/x86_64-apple-darwin/release/bundle/dmg/*.dmg",
    "openssl-cache-path": "native/build/openssl-macos14-x86_64/install",
    "openssl-prefix": "native/build/openssl-macos14-x86_64/install",
    "openssl-recipe": "native/build-openssl-macos.sh",
  },
  {
    runner: "windows-2022",
    target: "x86_64-pc-windows-msvc",
    bundle: "nsis",
    bundles: "nsis",
    artifact: "target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe",
    "openssl-cache-path": "native/build/x86_64-pc-windows-msvc/vcpkg",
    "openssl-recipe": "native/vcpkg.*",
  },
  {
    runner: "windows-11-arm",
    target: "aarch64-pc-windows-msvc",
    bundle: "nsis",
    bundles: "nsis",
    artifact: "target/aarch64-pc-windows-msvc/release/bundle/nsis/*.exe",
    "openssl-cache-path": "native/build/aarch64-pc-windows-msvc/vcpkg",
    "openssl-recipe": "native/vcpkg.*",
  },
  {
    runner: "ubuntu-22.04",
    target: "x86_64-unknown-linux-gnu",
    bundle: "deb",
    bundles: "deb,rpm,appimage",
    artifact:
      "target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb\ntarget/x86_64-unknown-linux-gnu/release/bundle/rpm/*.rpm\ntarget/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage\n",
    "extra-evidence":
      "target/x86_64-unknown-linux-gnu/release/bundle/rpm/*.cdx.json\ntarget/x86_64-unknown-linux-gnu/release/bundle/rpm/*.build.json\ntarget/x86_64-unknown-linux-gnu/release/bundle/appimage/*.cdx.json\ntarget/x86_64-unknown-linux-gnu/release/bundle/appimage/*.build.json\n",
    "openssl-cache-path": "native/build/unused-linux-openssl",
    "openssl-recipe": "native/openssl-probe/**",
  },
  {
    runner: "ubuntu-22.04-arm",
    target: "aarch64-unknown-linux-gnu",
    bundle: "deb",
    bundles: "deb,rpm,appimage",
    artifact:
      "target/aarch64-unknown-linux-gnu/release/bundle/deb/*.deb\ntarget/aarch64-unknown-linux-gnu/release/bundle/rpm/*.rpm\ntarget/aarch64-unknown-linux-gnu/release/bundle/appimage/*.AppImage\n",
    "extra-evidence":
      "target/aarch64-unknown-linux-gnu/release/bundle/rpm/*.cdx.json\ntarget/aarch64-unknown-linux-gnu/release/bundle/rpm/*.build.json\ntarget/aarch64-unknown-linux-gnu/release/bundle/appimage/*.cdx.json\ntarget/aarch64-unknown-linux-gnu/release/bundle/appimage/*.build.json\n",
    "openssl-cache-path": "native/build/unused-linux-openssl",
    "openssl-recipe": "native/openssl-probe/**",
  },
];

/** Select the existing platform definitions for one CI scope. */
export function desktopTargets(fullMatrix: boolean): DesktopTarget[] {
  if (fullMatrix) return targets;
  return targets.filter(({ target }) =>
    [
      "aarch64-apple-darwin",
      "x86_64-pc-windows-msvc",
      "x86_64-unknown-linux-gnu",
    ].includes(target),
  );
}

if (import.meta.main) {
  const [scope, ...extra] = process.argv.slice(2);
  if (extra.length || (scope !== "pr" && scope !== "full"))
    throw new Error("Pass exactly one desktop CI scope: pr or full.");
  const output = process.env.GITHUB_OUTPUT;
  if (!output)
    throw new Error("GITHUB_OUTPUT is required for the desktop matrix.");
  appendFileSync(
    output,
    `matrix=${JSON.stringify({ include: desktopTargets(scope === "full") })}\n`,
  );
}
