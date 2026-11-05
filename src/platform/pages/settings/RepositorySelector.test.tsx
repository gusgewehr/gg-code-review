import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import type { Repo } from "../../../types";
import { RepositorySelector } from "./RepositorySelector";

const repo = (id: number, fullName: string) =>
  ({ id, fullName, owner: "x", defaultBranch: "main", isPrivate: true }) as Repo;

async function render(props: Partial<Parameters<typeof RepositorySelector>[0]>) {
  const container = document.createElement("div");
  await act(async () =>
    createRoot(container).render(
      <RepositorySelector
        repos={[]}
        selected={new Set()}
        locked={new Set()}
        disabled={false}
        loading={false}
        hasOrganizations
        onToggle={() => {}}
        {...props}
      />,
    ),
  );
  return container;
}

describe("RepositorySelector", () => {
  it("keeps GitHub copy without forge", async () => {
    const c = await render({});
    expect(c.textContent).toContain("No repositories available. Refresh repository access to check again.");
    expect(c.querySelector("input")?.getAttribute("placeholder")).toBe("Search by organization or repository name…");
  });

  it("uses GitLab copy", async () => {
    const c = await render({ forge: "gitlab" });
    expect(c.textContent).toContain("No projects available. GitLab only lists projects where you have Developer");
    expect(c.querySelector("input")?.getAttribute("placeholder")).toBe("Search by group or project name…");
    const none = await render({ forge: "gitlab", hasOrganizations: false });
    expect(none.textContent).toContain("Select groups above to browse their projects.");
  });

  it("locks nested namespace projects", async () => {
    const c = await render({
      forge: "gitlab",
      repos: [repo(1, "g/sub/proj"), repo(2, "g/app")],
      locked: new Set(["g/sub"]),
    });
    const [a, b] = Array.from(c.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'));
    expect([a.checked, a.disabled, b.checked, b.disabled]).toEqual([true, true, false, false]);
    expect(a.getAttribute("aria-label")).toBe("g/sub/proj");
  });
});
