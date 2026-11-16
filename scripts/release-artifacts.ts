// Validate downloaded build evidence before handing any assets to gh release.
import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import type {
  SoftwareBill,
  SoftwareBuild,
} from "../apps/desktop/src/lib/software-bill.ts";

const targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
] as const;
type HashedFile = { file: string; sha256: string };
type Evidence = {
  build: SoftwareBuild;
  sbom: HashedFile;
  installers: HashedFile[];
};

function oneFile(directory: string, extension: string): string {
  const files = readdirSync(directory).filter((file) =>
    file.endsWith(extension),
  );
  if (files.length !== 1)
    throw new Error(`Expected one ${extension} in ${directory}`);
  if (/[\x00-\x1f\x7f]/.test(files[0]!))
    throw new Error("Invalid artifact filename");
  const path = join(directory, files[0]!);
  if (!lstatSync(path).isFile())
    throw new Error(`Artifact is not a regular file: ${path}`);
  return path;
}

function checkedFile(
  directory: string,
  file: unknown,
  digest: unknown,
): string {
  if (
    typeof file !== "string" ||
    file !== basename(file) ||
    /[\x00-\x1f\x7f]/.test(file) ||
    typeof digest !== "string" ||
    !/^[0-9a-f]{64}$/.test(digest)
  )
    throw new Error("Invalid artifact filename or hash");
  const path = join(directory, file);
  if (
    !lstatSync(path).isFile() ||
    createHash("sha256").update(readFileSync(path)).digest("hex") !== digest
  )
    throw new Error(`Artifact hash mismatch: ${path}`);
  return path;
}

/** Collect the five native release targets, rejecting missing/mixed/tampered artifacts. */
export function collectReleaseArtifacts(
  directory: string,
  version: string,
): string[] {
  const assets: string[] = [];
  for (const target of targets) {
    const root = join(resolve(directory), `desktop-${target}`);
    const formats = target.includes("linux")
      ? [
          ["deb", ".deb"],
          ["rpm", ".rpm"],
          ["appimage", ".AppImage"],
        ]
      : [["", target.includes("apple") ? ".dmg" : ".exe"]];
    const canonical = join(root, formats[0]![0]!);
    const evidencePath = oneFile(canonical, ".build.json");
    const evidenceBytes = readFileSync(evidencePath);
    const evidence = JSON.parse(evidenceBytes.toString()) as Evidence;
    const context = evidence?.build;
    if (
      context?.target !== target ||
      context.native !== true ||
      context.profile !== "release" ||
      context.kind !== "desktop" ||
      !Array.isArray(evidence?.installers) ||
      evidence.installers.length !== formats.length
    )
      throw new Error(`Build evidence does not match release target ${target}`);
    const sbomPath = checkedFile(
      canonical,
      evidence.sbom?.file,
      evidence.sbom?.sha256,
    );
    const sbomBytes = readFileSync(sbomPath);
    const bill = JSON.parse(sbomBytes.toString()) as SoftwareBill;
    if (
      bill?.metadata?.component?.version !== version ||
      !Array.isArray(bill.metadata.properties)
    )
      throw new Error(`SBOM version does not match release ${version}`);
    for (const [name, value] of Object.entries(context)) {
      if (
        !bill.metadata.properties.some(
          (property) =>
            property?.name === `ruf:build:${name}` &&
            property.value === String(value),
        )
      )
        throw new Error(`SBOM context does not match ${target}`);
    }
    const consumed = new Set<string>();
    for (const [folder, extension] of formats) {
      const bundle = join(root, folder!);
      const installer = oneFile(bundle, extension!);
      const recorded = evidence.installers.find(
        (entry) => entry?.file === basename(installer),
      );
      if (!recorded || consumed.has(recorded.file))
        throw new Error(`Unrecorded installer ${installer}`);
      consumed.add(recorded.file);
      assets.push(checkedFile(bundle, recorded.file, recorded.sha256));
      if (
        !readFileSync(oneFile(bundle, ".build.json")).equals(evidenceBytes) ||
        !readFileSync(
          checkedFile(bundle, evidence.sbom.file, evidence.sbom.sha256),
        ).equals(sbomBytes)
      )
        throw new Error(
          `Different evidence between bundle formats for ${target}`,
        );
    }
    assets.push(sbomPath, evidencePath);
  }
  if (
    new Set(assets.map((file) => basename(file).toLowerCase())).size !==
    assets.length
  )
    throw new Error("Release asset names collide between architectures");
  return assets;
}

if (import.meta.main) {
  const [directory, tag, ...extra] = process.argv.slice(2);
  if (!directory || !tag || extra.length)
    throw new Error("Pass artifacts directory and release tag");
  for (const file of collectReleaseArtifacts(directory, tag.replace(/^v/, "")))
    console.log(file);
}
