import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const shellTest = process.platform === "win32" ? test.skip : test;
for (const [architecture, configure, folder] of [
  ["arm64", "darwin64-arm64-cc", "openssl-macos14"],
  ["x86_64", "darwin64-x86_64-cc", "openssl-macos14-x86_64"],
] as const) {
  shellTest(
    `macOS OpenSSL recipe selects ${architecture} without reusing another CPU's prefix`,
    () => {
      const directory = mkdtempSync(join(tmpdir(), "ruf-openssl-recipe-"));
      try {
        mkdirSync(join(directory, "native"));
        const bin = join(directory, "bin");
        mkdirSync(bin);
        copyFileSync(
          resolve("native/build-openssl-macos.sh"),
          join(directory, "native/build.sh"),
        );
        const command = (name: string, body: string) => {
          const path = join(bin, name);
          writeFileSync(path, `#!/bin/bash\nset -e\n${body}\n`);
          chmodSync(path, 0o755);
        };
        command(
          "uname",
          `if [[ "$1" == -s ]]; then echo Darwin; else echo ${architecture}; fi`,
        );
        command("curl", "exit 0");
        command("shasum", "cat >/dev/null");
        command(
          "tar",
          `for name in openssl-macos14 openssl-macos14-x86_64; do mkdir -p "$RUF_RECIPE_FIXTURE/native/build/$name/openssl-3.5.9" "$RUF_RECIPE_FIXTURE/native/build/$name/install"; echo 'synthetic license' > "$RUF_RECIPE_FIXTURE/native/build/$name/openssl-3.5.9/LICENSE.txt"; done`,
        );
        command(
          "perl",
          `printf '%s\\n' "$*" >> "$RUF_RECIPE_FIXTURE/configure.log"`,
        );
        command(
          "make",
          `printf '%s\\n' "$*" >> "$RUF_RECIPE_FIXTURE/make.log"`,
        );
        const result = spawnSync("bash", [join(directory, "native/build.sh")], {
          encoding: "utf8",
          env: {
            ...process.env,
            PATH: `${bin}:${process.env.PATH}`,
            RUF_RECIPE_FIXTURE: directory,
          },
        });
        expect(result.status, result.stderr).toBe(0);
        const args = readFileSync(join(directory, "configure.log"), "utf8");
        expect(args).toContain(`Configure ${configure} `);
        expect(args).toContain(
          `--prefix=${directory}/native/build/${folder}/install`,
        );
        expect(args).toContain("-mmacosx-version-min=14.0");
        expect(readFileSync(join(directory, "make.log"), "utf8")).toContain(
          "test HARNESS_JOBS=4",
        );
      } finally {
        rmSync(directory, { recursive: true, force: true });
      }
    },
  );
}

shellTest(
  "native SDK refuses a mismatched Rust host before compiling or staging libraries",
  () => {
    const directory = mkdtempSync(join(tmpdir(), "ruf-sdk-host-"));
    try {
      mkdirSync(join(directory, "native"));
      mkdirSync(join(directory, "bin"));
      for (const file of ["build.ts", "vcpkg.ts", "sources.json"])
        copyFileSync(resolve("native", file), join(directory, "native", file));
      const rustc = join(directory, "bin/rustc");
      writeFileSync(rustc, "#!/bin/sh\necho 'host: unsupported-native-host'\n");
      chmodSync(rustc, 0o755);
      const result = spawnSync(
        process.execPath,
        [join(directory, "native/build.ts")],
        {
          encoding: "utf8",
          env: {
            ...process.env,
            PATH: `${join(directory, "bin")}:${process.env.PATH}`,
          },
        },
      );
      expect(result.status).not.toBe(0);
      expect(result.stderr).toContain("Native SDK requires matching Rust host");
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  },
);
