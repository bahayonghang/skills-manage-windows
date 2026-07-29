/// <reference types="node" />

import { describe, expect, it } from "vitest";

type ReleaseAssetHelpers = {
  expectedReleaseAssets: (version: string, includeArm64?: boolean) => string[];
};
type ReleaseStateHelpers = {
  validateReleaseState: (
    release: ReleaseFixture,
    expected: { tag: string; version: string; expectDraft?: boolean },
  ) => number;
};
type ReleaseFixture = {
  id: number;
  draft: boolean;
  tag_name: string;
  target_commitish: string;
  assets: Array<{ name: string; size: number }>;
};

// @ts-expect-error The release helpers are ESM Node scripts outside the TS source tree.
const artifactModule = await import("../../../scripts/release-artifacts.mjs");
// @ts-expect-error The release helpers are ESM Node scripts outside the TS source tree.
const stateModule = await import("../../../scripts/release-draft-state.mjs");
const { expectedReleaseAssets } = artifactModule as ReleaseAssetHelpers;
const { validateReleaseState } = stateModule as ReleaseStateHelpers;

function fixture(draft = true): ReleaseFixture {
  return {
    id: 42,
    draft,
    tag_name: "v1.2.3",
    target_commitish: "main",
    assets: [...expectedReleaseAssets("1.2.3"), "SHA256SUMS"].map((name) => ({ name, size: 10 })),
  };
}

describe("release draft API state", () => {
  it("accepts complete draft and public states", () => {
    expect(validateReleaseState(fixture(), { tag: "v1.2.3", version: "1.2.3" }))
      .toBe(42);
    expect(validateReleaseState(fixture(false), {
      tag: "v1.2.3",
      version: "1.2.3",
      expectDraft: false,
    })).toBe(42);
  });

  it("rejects public state before publish and wrong tag or inventory", () => {
    expect(() =>
      validateReleaseState(fixture(false), { tag: "v1.2.3", version: "1.2.3" }),
    ).toThrow(/draft state mismatch/);

    const wrongTag = fixture();
    wrongTag.tag_name = "v9.9.9";
    expect(() =>
      validateReleaseState(wrongTag, { tag: "v1.2.3", version: "1.2.3" }),
    ).toThrow(/tag mismatch/);

    const emptyAsset = fixture();
    emptyAsset.assets[0].size = 0;
    expect(() =>
      validateReleaseState(emptyAsset, { tag: "v1.2.3", version: "1.2.3" }),
    ).toThrow(/empty asset/);
  });
});
