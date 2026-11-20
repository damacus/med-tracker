import assert from "node:assert/strict";
import test from "node:test";
import { previousReleaseImage } from "./upgrade_release.mjs";

test("upgrade baseline uses a published stable release and excludes the release under test", () => {
  const releases = [
    { tagName: "v0.6.9", isDraft: true },
    { tagName: "v0.6.11-rc.1", isPrerelease: true },
    { tagName: "v0.6.10" },
    { tagName: "v0.6.8" },
  ];
  assert.equal(previousReleaseImage(releases, ""), "ghcr.io/damacus/med-tracker:0.6.10");
  assert.equal(previousReleaseImage(releases, "v0.6.10"), "ghcr.io/damacus/med-tracker-loco:0.6.8");
});

test("upgrade baseline compares versions numerically and refuses a missing baseline", () => {
  assert.equal(previousReleaseImage([{ tagName: "v0.6.8" }, { tagName: "v0.6.10" }], ""), "ghcr.io/damacus/med-tracker:0.6.10");
  assert.throws(() => previousReleaseImage([{ tagName: "v0.6.10", isDraft: true }], ""), /published stable/);
  assert.throws(() => previousReleaseImage([{ tagName: "v0.6.10\n" }], ""), /published stable/);
});

test("rebuilding an older tag never selects a newer baseline", () => {
  assert.equal(previousReleaseImage([{ tagName: "v0.6.11" }, { tagName: "v0.6.10" }, { tagName: "v0.6.8" }], "v0.6.10"), "ghcr.io/damacus/med-tracker-loco:0.6.8");
});
