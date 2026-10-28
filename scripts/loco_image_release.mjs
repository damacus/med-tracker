import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function releaseVersion(tag, revision, checkedRevision, cargoVersion) {
  if (!/^[a-f0-9]{40}$/.test(revision) || revision !== checkedRevision) {
    throw new Error("Release commit differs from the workflow commit");
  }
  if (!tag) return "";
  const match = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(tag);
  if (!match || match[0] !== tag || tag.length > 128 || match[4]?.split(".").some((part) => /^0\d+$/.test(part))) {
    throw new Error("Release tag must be v-prefixed SemVer without build metadata");
  }
  const version = tag.slice(1);
  if (version !== cargoVersion) throw new Error("Release tag differs from Cargo package version");
  return version;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const cargo = readFileSync("Cargo.toml", "utf8").match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = releaseVersion(
    process.env.RELEASE_TAG ?? "",
    process.env.RELEASE_REVISION,
    execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
    cargo?.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
  );
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `release_version=${version}\n`);
  console.log(version ? `Validated release ${version}` : "Validated SHA-only image build");
}
