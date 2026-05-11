import { describe, expect, it } from "vitest";

import {
  defaultCentralViewState,
  parseCentralViewState,
  parseCentralViewStateFromUrl,
  serializeCentralViewState,
  toQueryString,
  updateCentralViewState,
} from "@/lib/centralViewState";

describe("centralViewState", () => {
  it("default state is sane", () => {
    const s = defaultCentralViewState();
    expect(s).toEqual({
      q: "",
      repos: [],
      tags: [],
      view: "grid",
      group: "none",
      sortField: "name",
      sortDir: "asc",
    });
  });

  it("parses empty params to default", () => {
    const s = parseCentralViewState(new URLSearchParams());
    expect(s).toEqual(defaultCentralViewState());
  });

  it("parses csv arrays", () => {
    const s = parseCentralViewState(new URLSearchParams("repos=r1,r2&tags=t1"));
    expect(s.repos).toEqual(["r1", "r2"]);
    expect(s.tags).toEqual(["t1"]);
  });

  it("parses sort with colon", () => {
    const s = parseCentralViewState(new URLSearchParams("sort=updatedAt:desc"));
    expect(s.sortField).toBe("updatedAt");
    expect(s.sortDir).toBe("desc");
  });

  it("falls back when sort is malformed", () => {
    const s = parseCentralViewState(new URLSearchParams("sort=bogus:asc"));
    expect(s.sortField).toBe("name"); // default
  });

  it("ignores unknown view / group values", () => {
    const s = parseCentralViewState(new URLSearchParams("view=foo&group=bar"));
    expect(s.view).toBe("grid");
    expect(s.group).toBe("none");
  });

  it("preserves savedView when present", () => {
    const s = parseCentralViewState(new URLSearchParams("savedView=v1"));
    expect(s.savedView).toBe("v1");
  });

  it("serializes only non-default fields", () => {
    const params = serializeCentralViewState({
      ...defaultCentralViewState(),
      q: "tag:editor",
      repos: ["r1", "r2"],
    });
    expect(params.toString()).toBe("q=tag%3Aeditor&repos=r1%2Cr2");
  });

  it("round-trips through URL", () => {
    const original = updateCentralViewState(defaultCentralViewState(), {
      q: "tag:editor paper",
      repos: ["r1", "r2"],
      tags: ["t1"],
      view: "list",
      group: "tag",
      sortField: "updatedAt",
      sortDir: "desc",
    });
    const url = `https://app.local/${toQueryString(original)}`;
    const parsed = parseCentralViewStateFromUrl(url);
    expect(parsed).toEqual(original);
  });

  it("toQueryString returns empty string for default state", () => {
    expect(toQueryString(defaultCentralViewState())).toBe("");
  });

  it("trims and drops empty csv parts", () => {
    const s = parseCentralViewState(new URLSearchParams("repos= r1 , ,r2 "));
    expect(s.repos).toEqual(["r1", "r2"]);
  });
});
