import { QueryClient, QueryClientContext, useQuery } from "@tanstack/react-query";
import { useContext } from "react";
import { setupApi } from "../../services/setupApi";

export type ForgeKind = "github" | "gitlab";

export function forgeTerms(forge: ForgeKind) {
  const gitlab = forge === "gitlab";
  return {
    forge,
    platform: gitlab ? "GitLab" : "GitHub",
    pr: gitlab ? "merge request" : "pull request",
    prs: gitlab ? "merge requests" : "pull requests",
    Prs: gitlab ? "Merge requests" : "Pull requests",
    short: gitlab ? "MR" : "PR",
    shorts: gitlab ? "MRs" : "PRs",
    // Phase 4: GitLab never approves or requests changes.
    verdicts: !gitlab,
    ref: (n: number | string) => `${gitlab ? "!" : "#"}${n}`,
  };
}

// Only so the hook is valid without a provider: disabled, never fetches.
const idleClient = new QueryClient();

export function useForgeTerms() {
  const client = useContext(QueryClientContext);
  const { data } = useQuery(
    {
      queryKey: ["forge", "status"],
      queryFn: () => setupApi.forgeStatus(),
      retry: false,
      staleTime: Number.POSITIVE_INFINITY,
      enabled: client !== undefined,
    },
    client ?? idleClient,
  );
  return forgeTerms(data?.provider === "gitlab" ? "gitlab" : "github");
}
