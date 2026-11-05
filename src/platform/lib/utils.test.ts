import { describe, expect, it } from "vitest";
import type { Repo } from "../../types";
import { repoOwner } from "./utils";

const r = (fullName: string, owner: string) => ({ fullName, owner }) as Repo;

describe("repoOwner", () => {
  it("returns the namespace before the last slash", () => {
    expect(repoOwner(r("o/r", "o"))).toBe("o");
    expect(repoOwner(r("g/sub/proj", "g/sub"))).toBe("g/sub");
    expect(repoOwner(r("a/b/c/d", "x"))).toBe("a/b/c");
  });
  it("falls back to owner without a namespace", () => {
    expect(repoOwner(r("noslash", "fallback"))).toBe("fallback");
    expect(repoOwner(r("/x", "fallback"))).toBe("fallback");
  });
});
