import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function previousReleaseImage(releases, currentTag) {
  const compare = (a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
  const currentVersion = /^v(\d+)\.(\d+)\.(\d+)(?:-|$)/.exec(currentTag)?.slice(1).map(Number);
  if (currentTag && !currentVersion) throw new Error("Invalid release tag for the upgrade check");
  const candidates = releases.flatMap(release => {
    const match = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(release.tagName ?? "");
    if (!match || match[0] !== release.tagName || release.isDraft || release.isPrerelease || release.tagName === currentTag) return [];
    const version = match.slice(1).map(Number);
    if (currentVersion && compare(version, currentVersion) >= 0) return [];
    return [{ tag: release.tagName, version }];
  });
  candidates.sort((a, b) => compare(b.version, a.version));
  const previous = candidates[0];
  if (!previous) throw new Error("No published stable release is available for the upgrade check");
  const [major, minor, patch] = previous.version;
  const repository = major === 0 && (minor < 6 || (minor === 6 && patch < 10)) ? "med-tracker-loco" : "med-tracker";
  return `ghcr.io/damacus/${repository}:${previous.tag.slice(1)}`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const image = previousReleaseImage(JSON.parse(readFileSync(process.argv[2], "utf8")), process.env.RELEASE_TAG ?? "");
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `image=${image}\n`);
  console.log(`Upgrade baseline: ${image}`);
}
