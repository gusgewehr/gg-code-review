import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { forgeTerms, useForgeTerms } from "./forgeTerms";

vi.mock("../../services/setupApi", () => ({ setupApi: { forgeStatus: () => new Promise(() => undefined) } }));

function Probe() {
  const t = useForgeTerms();
  return <span>{`${t.platform}|${t.pr}|${t.ref(3)}`}</span>;
}

describe("forgeTerms", () => {
  it("pins the github strings", () => {
    const { ref, ...rest } = forgeTerms("github");
    expect(rest).toEqual({
      forge: "github",
      platform: "GitHub",
      pr: "pull request",
      prs: "pull requests",
      Prs: "Pull requests",
      short: "PR",
      shorts: "PRs",
      verdicts: true,
    });
    expect(ref(12)).toBe("#12");
  });

  it("returns gitlab terms", () => {
    const t = forgeTerms("gitlab");
    expect([t.platform, t.pr, t.prs, t.Prs, t.short, t.shorts, t.verdicts, t.ref(12)]).toEqual([
      "GitLab",
      "merge request",
      "merge requests",
      "Merge requests",
      "MR",
      "MRs",
      false,
      "!12",
    ]);
  });

  it("defaults to github without a provider and while pending", () => {
    expect(renderToStaticMarkup(<Probe />)).toContain("GitHub|pull request|#3");
    const client = new QueryClient();
    expect(
      renderToStaticMarkup(
        <QueryClientProvider client={client}>
          <Probe />
        </QueryClientProvider>,
      ),
    ).toContain("GitHub|pull request|#3");
  });

  it("uses gitlab when the cached forge status says so", () => {
    const client = new QueryClient();
    client.setQueryData(["forge", "status"], { provider: "gitlab", host: "g.example.com" });
    expect(
      renderToStaticMarkup(
        <QueryClientProvider client={client}>
          <Probe />
        </QueryClientProvider>,
      ),
    ).toContain("GitLab|merge request|!3");
  });
});
