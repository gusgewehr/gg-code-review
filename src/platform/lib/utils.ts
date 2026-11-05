import type { Repo } from "../../types";

export function repoOwner(repo: Repo) {
  const i = repo.fullName.lastIndexOf("/");
  return i > 0 ? repo.fullName.slice(0, i) : repo.owner;
}

export function toggleSet<T>(current: Set<T>, value: T) {
  const next = new Set(current);
  next.has(value) ? next.delete(value) : next.add(value);
  return next;
}

export function formatError(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

/** Unwraps a value that a query's `enabled` flag already guarantees, failing loudly if it is missing. */
export function required<T>(value: T | null | undefined, name: string): T {
  if (value === null || value === undefined) throw new Error(`Missing ${name}`);
  return value;
}
