// Run synthetic Linux OS-service checks without touching the current desktop session.
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

if (process.platform !== "linux")
  throw new Error("test:linux requires a Linux host");
const directory = mkdtempSync(join(tmpdir(), "ruf-linux-"));
try {
  execFileSync(
    "dbus-run-session",
    ["--", "xvfb-run", "-a", "bash", "tests/interop/linux/session.sh"],
    {
      cwd: resolve(import.meta.dir, ".."),
      stdio: "inherit",
      timeout: 600_000,
      env: {
        ...process.env,
        RUF_LINUX_TEST_SESSION: "1",
        XDG_CONFIG_HOME: join(directory, "config"),
        XDG_DATA_HOME: join(directory, "data"),
        XDG_RUNTIME_DIR: directory,
        PULSE_SERVER: `unix:${directory}/pulse-native`,
      },
    },
  );
} finally {
  rmSync(directory, { recursive: true, force: true });
}
