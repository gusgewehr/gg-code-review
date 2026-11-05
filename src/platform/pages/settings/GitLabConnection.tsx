import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { DSButton, DSInput } from "../../../components/DesignSystem";
import { setupApi } from "../../../services/setupApi";
import type { ForgeStatus } from "../../../types";
import { formatError } from "../../lib/utils";

export function GitLabConnection({ status }: { status: ForgeStatus }) {
  const queryClient = useQueryClient();
  const [token, setToken] = useState("");
  const [pending, setPending] = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["forge", "status"] });
  // No secret in this mutation, so its state is safe to keep.
  const disconnect = useMutation({ mutationFn: () => setupApi.gitlabDisconnect(), onSuccess: refresh });

  // Not useMutation: its `variables` would keep the token in the mutation cache.
  async function connect() {
    const value = token;
    setToken("");
    setPending(true);
    setConnectError(null);
    try {
      await setupApi.gitlabConnect(value);
    } catch (error) {
      setConnectError(formatError(error));
    } finally {
      setPending(false);
      void refresh();
    }
  }

  const error = connectError ?? status.error?.message;
  const row = "flex items-center justify-between gap-4 text-sm";
  return (
    <div className="grid gap-3 rounded-lg border border-line bg-[color-mix(in_srgb,var(--surface-2)_70%,transparent)] p-4">
      <div>
        <div className="text-sm font-semibold">GitLab connection</div>
        <div className="mt-1 text-sm text-fg-muted">
          Connect with a personal access token to review merge requests on {status.host}.
        </div>
      </div>
      <div className={row}>
        <span className="text-fg-muted">Host</span>
        <span className="font-mono">{status.host}</span>
      </div>
      {status.loggedIn ? (
        <>
          <div className={row}>
            <span className="text-fg-muted">User</span>
            <span className="font-mono">{status.login}</span>
          </div>
          <div className={row}>
            <span className="text-fg-muted">Token expires</span>
            <span className="font-mono">{status.expiresAt ?? "No expiry"}</span>
          </div>
          <div className="text-sm font-semibold text-success">Connected</div>
        </>
      ) : (
        <>
          <div className="text-sm text-fg-muted">
            Not connected. Paste a personal access token with the `api` scope and select Connect.
          </div>
          <DSInput
            type="password"
            autoComplete="off"
            spellCheck={false}
            aria-label="Personal access token"
            value={token}
            onChange={(event) => setToken(event.target.value)}
          />
          <div className="text-sm text-fg-muted">Needs the `api` scope.</div>
        </>
      )}
      {error && <div className="text-sm font-semibold text-danger">{error}</div>}
      <div className="flex justify-end gap-3">
        {status.loggedIn ? (
          <DSButton className="text-danger" onClick={() => disconnect.mutate()} disabled={disconnect.isPending}>
            Disconnect
          </DSButton>
        ) : (
          <DSButton tone="primary" onClick={() => void connect()} disabled={!token.trim() || pending}>
            {pending ? "Connecting" : "Connect"}
          </DSButton>
        )}
      </div>
    </div>
  );
}
