import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setupApi } from "../../../services/setupApi";
import type { ForgeStatus } from "../../../types";
import { GitLabConnection } from "./GitLabConnection";

vi.mock("../../../services/setupApi", () => ({ setupApi: { gitlabConnect: vi.fn(), gitlabDisconnect: vi.fn() } }));

const TOKEN = "glpat-TESTTOKEN0000000000";
const base: ForgeStatus = {
  provider: "gitlab",
  host: "gitlab.example.com",
  loggedIn: false,
  login: null,
  expiresAt: null,
  error: null,
};
let client: QueryClient;
let container: HTMLDivElement;
let root: Root;

async function render(status: ForgeStatus) {
  await act(async () =>
    root.render(
      <QueryClientProvider client={client}>
        <GitLabConnection status={status} />
      </QueryClientProvider>,
    ),
  );
}
const button = (label: string) =>
  Array.from(container.querySelectorAll("button")).find((b) => b.textContent === label) as HTMLButtonElement;
const input = () => container.querySelector("input") as HTMLInputElement;
async function type(value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input(), value);
    input().dispatchEvent(new Event("input", { bubbles: true }));
  });
}

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.mocked(setupApi.gitlabConnect).mockReset();
  vi.mocked(setupApi.gitlabDisconnect).mockReset().mockResolvedValue({ ok: true });
  client = new QueryClient();
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.unstubAllGlobals();
});

describe("GitLabConnection", () => {
  it("renders the disconnected state with Connect disabled while empty", async () => {
    await render(base);
    expect(container.textContent).toContain("GitLab connection");
    expect(container.textContent).toContain("Not connected. Paste a personal access token");
    expect(input().type).toBe("password");
    expect(button("Connect").disabled).toBe(true);
  });

  it("clears the token on click and calls gitlabConnect once", async () => {
    let finish: (s: ForgeStatus) => void = () => undefined;
    vi.mocked(setupApi.gitlabConnect).mockReturnValue(new Promise((r) => (finish = r)));
    await render(base);
    await type(TOKEN);
    await act(async () => button("Connect").click());
    expect(input().value).toBe("");
    expect(setupApi.gitlabConnect).toHaveBeenCalledTimes(1);
    expect(setupApi.gitlabConnect).toHaveBeenCalledWith(TOKEN);
    await act(async () => finish({ ...base, loggedIn: true }));
  });

  it("shows server errors without leaking the token into DOM, caches or storage", async () => {
    vi.mocked(setupApi.gitlabConnect).mockRejectedValue(
      new Error("Token is missing the `api` scope. Create a new token with `api` and try again."),
    );
    await render(base);
    await type(TOKEN);
    await act(async () => button("Connect").click());
    expect(container.textContent).toContain("Token is missing the `api` scope.");
    expect(document.body.innerHTML).not.toContain(TOKEN);
    expect(
      JSON.stringify(
        client
          .getQueryCache()
          .getAll()
          .map((q) => q.state),
      ),
    ).not.toContain(TOKEN);
    expect(
      JSON.stringify(
        client
          .getMutationCache()
          .getAll()
          .map((m) => m.state),
      ),
    ).not.toContain(TOKEN);
    expect(JSON.stringify(Object.entries(window.localStorage ?? {}))).not.toContain(TOKEN);
    expect(JSON.stringify(Object.entries(window.sessionStorage ?? {}))).not.toContain(TOKEN);
  });

  it("renders status.error on load", async () => {
    await render({
      ...base,
      error: { code: "expired", message: "This token has expired. Create a new token and try again." },
    });
    expect(container.textContent).toContain("This token has expired.");
  });

  it("shows connected details and disconnects", async () => {
    await render({ ...base, loggedIn: true, login: "alice", expiresAt: null });
    expect(container.textContent).toContain("alice");
    expect(container.textContent).toContain("No expiry");
    expect(container.textContent).toContain("Connected");
    await act(async () => button("Disconnect").click());
    expect(setupApi.gitlabDisconnect).toHaveBeenCalled();
  });
});
