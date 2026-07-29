import { describe, it, expect, afterEach } from "vitest";
import {
  clearReturnContext,
  clearScrollPosition,
  consumeReturnContext,
  consumeScrollPosition,
  createScrollRestorationState,
  readReturnContext,
  readScrollPosition,
  saveReturnContext,
  saveScrollPosition,
} from "@/lib/scrollRestoration";

// ─── Scroll map ───────────────────────────────────────────────────────────────

describe("scroll position map", () => {
  afterEach(() => {
    clearScrollPosition("test-key-1");
    clearScrollPosition("test-key-2");
  });

  it("returns null when reading a key that was never written", () => {
    expect(readScrollPosition("test-key-1")).toBeNull();
  });

  it("stores and reads back a scroll position", () => {
    saveScrollPosition("test-key-1", 320);
    expect(readScrollPosition("test-key-1")).toBe(320);
  });

  it("consume returns the value and then removes the entry (single-use)", () => {
    saveScrollPosition("test-key-1", 440);
    expect(consumeScrollPosition("test-key-1")).toBe(440);
    expect(consumeScrollPosition("test-key-1")).toBeNull();
  });

  it("different keys do not clobber each other", () => {
    saveScrollPosition("test-key-1", 100);
    saveScrollPosition("test-key-2", 200);
    expect(readScrollPosition("test-key-1")).toBe(100);
    expect(readScrollPosition("test-key-2")).toBe(200);
  });

  it("createScrollRestorationState builds the expected shape", () => {
    expect(createScrollRestorationState("foo", 50)).toEqual({
      key: "foo",
      scrollTop: 50,
    });
  });
});

// ─── Return-context map ───────────────────────────────────────────────────────

describe("return-context map", () => {
  afterEach(() => {
    clearReturnContext("collections");
    clearReturnContext("scope-a");
    clearReturnContext("scope-b");
  });

  it("returns null for an unknown scope", () => {
    expect(readReturnContext("collections")).toBeNull();
  });

  it("stores and reads back a context object", () => {
    saveReturnContext("collections", { collectionId: "col-7" });
    expect(readReturnContext("collections")).toEqual({ collectionId: "col-7" });
  });

  it("consume returns the context and then removes the entry", () => {
    saveReturnContext("collections", { collectionId: "col-9" });
    expect(consumeReturnContext("collections")).toEqual({
      collectionId: "col-9",
    });
    expect(consumeReturnContext("collections")).toBeNull();
  });

  it("saving under the same scope overwrites the previous context", () => {
    saveReturnContext("scope-a", { marker: 1 });
    saveReturnContext("scope-a", { marker: 2 });
    expect(readReturnContext("scope-a")).toEqual({ marker: 2 });
  });

  it("different scopes are isolated from each other", () => {
    saveReturnContext("scope-a", { value: "alpha" });
    saveReturnContext("scope-b", { value: "beta" });
    expect(readReturnContext("scope-a")).toEqual({ value: "alpha" });
    expect(readReturnContext("scope-b")).toEqual({ value: "beta" });
    // Consuming one does not affect the other.
    consumeReturnContext("scope-a");
    expect(readReturnContext("scope-b")).toEqual({ value: "beta" });
  });
});
