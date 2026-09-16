import { useState, useEffect } from "react";
import { invoke } from "../lib/tauri.ts";
import { useAuth } from "../lib/auth";

interface SubscriptionInfo {
  plan: string;
  used: number;
  limit: number;
  reset_date: string;
}

interface PricingModel {
  model_name: string;
  description?: string;
  tags?: string;
  supported_endpoint_types?: string[];
}

/** NAPI model.Log JSON (`NAPI/model/log.go`). */
interface UserLog {
  id?: number;
  model_name?: string;
  created_at?: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  quota?: number;
  token_name?: string;
  type?: number;
}

const LOG_TYPE_CONSUME = 2;

interface WeeklyUsage {
  start: number;
  end: number;
  usedUsd: number;
  usedQuota: number;
  requestCount: number;
  tokenUsed: number;
}

function normalizeWeekly(raw: unknown): WeeklyUsage | null {
  const rec = asRecord(raw);
  if (!rec) return null;
  const n = (v: unknown) =>
    typeof v === "number" && Number.isFinite(v) ? v : 0;
  return {
    start: n(rec.start),
    end: n(rec.end),
    usedUsd: n(rec.usedUsd ?? rec.used_usd),
    usedQuota: n(rec.usedQuota ?? rec.used_quota),
    requestCount: n(rec.requestCount ?? rec.request_count),
    tokenUsed: n(rec.tokenUsed ?? rec.token_used),
  };
}

function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

function asRecord(v: unknown): Record<string, unknown> | null {
  return v !== null && typeof v === "object" && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : null;
}

function asNumber(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) ? v : null;
}

/** NAPI `common.ApiSuccess` + `PageInfo`: `{ success, data: { page, page_size, total, items } }`. */
function parseUserLogs(res: unknown): {
  items: UserLog[];
  total: number | null;
  error: string | null;
} {
  const root = asRecord(res);
  if (!root) {
    return { items: [], total: null, error: "Unexpected logs response" };
  }
  if (root.success === false) {
    const message =
      typeof root.message === "string" && root.message.trim()
        ? root.message
        : "Couldn't load request history";
    return { items: [], total: null, error: message };
  }

  const data = asRecord(root.data);
  const rawItems = data?.items ?? root.items;
  const items = Array.isArray(rawItems) ? (rawItems as UserLog[]) : [];
  const total = asNumber(data?.total) ?? asNumber(root.total);

  const consume = items.filter(
    (row) => typeof row.type !== "number" || row.type === LOG_TYPE_CONSUME,
  );
  // Client-side consume filter: only trust pageInfo.total when this page
  // did not drop non-consume rows (otherwise total includes topups/etc).
  const dropped = items.length - consume.length;
  return {
    items: consume,
    total: dropped === 0 ? total : null,
    error: null,
  };
}

function parseQuotaPerUnit(res: unknown): number | null {
  const root = asRecord(res);
  const data = asRecord(root?.data) ?? root;
  const v = asNumber(data?.quota_per_unit);
  return v !== null && v > 0 ? v : null;
}

function formatLogTime(ts?: number): string {
  if (typeof ts !== "number" || ts <= 0) return "—";
  const d = new Date(ts * 1000);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString();
}

function formatCompactCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return n.toLocaleString();
}

function formatLogQuota(raw: number | undefined, perUnit: number | null): string {
  if (typeof raw !== "number" || !Number.isFinite(raw)) return "—";
  if (perUnit && perUnit > 0) {
    const usd = raw / perUnit;
    const digits = usd >= 1 ? 2 : usd >= 0.01 ? 4 : 6;
    return `$${usd.toFixed(digits)}`;
  }
  return `${raw.toLocaleString()} quota units`;
}

export function SubscriptionPage() {
  const { state } = useAuth();
  const [sub, setSub] = useState<SubscriptionInfo | null>(null);
  const [models, setModels] = useState<PricingModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modelsQ, setModelsQ] = useState("");
  const [logs, setLogs] = useState<UserLog[]>([]);
  const [logsTotal, setLogsTotal] = useState<number | null>(null);
  const [logsError, setLogsError] = useState<string | null>(null);
  const [logsLoading, setLogsLoading] = useState(false);
  const [quotaPerUnit, setQuotaPerUnit] = useState<number | null>(null);
  const [weekly, setWeekly] = useState<WeeklyUsage | null>(null);
  const [weeklyError, setWeeklyError] = useState<string | null>(null);

  const loadLogs = () => {
    if (!state.authed) return;
    setLogsLoading(true);
    setLogsError(null);
    invoke<unknown>("fetch_user_logs", { baseUrl: state.baseUrl })
      .then((res) => {
        const parsed = parseUserLogs(res);
        if (parsed.error) {
          setLogs([]);
          setLogsTotal(null);
          setLogsError(parsed.error);
          return;
        }
        setLogs(parsed.items);
        setLogsTotal(parsed.total);
      })
      .catch((e) => {
        setLogs([]);
        setLogsTotal(null);
        setLogsError(errorText(e));
      })
      .finally(() => setLogsLoading(false));
  };

  const load = () => {
    if (!state.authed) return;
    setLoading(true);
    setError(null);
    invoke<SubscriptionInfo>("fetch_subscription", {
      baseUrl: state.baseUrl,
    })
      .then((info) => {
        setSub(info);
        setError(null);
      })
      .catch((e) => setError(errorText(e)))
      .finally(() => setLoading(false));
    loadLogs();
    setWeeklyError(null);
    invoke<unknown>("fetch_weekly_usage", { baseUrl: state.baseUrl })
      .then((res) => {
        setWeekly(normalizeWeekly(res));
        setWeeklyError(null);
      })
      .catch((e) => {
        setWeekly(null);
        setWeeklyError(errorText(e));
      });
  };

  useEffect(() => {
    load();
  }, [state.authed, state.baseUrl]);

  // ponytail: the catalog is secondary — a failure here must not blank the quota view.
  useEffect(() => {
    if (!state.authed) return;
    invoke<{ data: PricingModel[] }>("get_pricing", { baseUrl: state.baseUrl })
      .then((res) => setModels(Array.isArray(res?.data) ? res.data : []))
      .catch(() => setModels([]));
    invoke<unknown>("get_status", { baseUrl: state.baseUrl })
      .then((res) => setQuotaPerUnit(parseQuotaPerUnit(res)))
      .catch(() => setQuotaPerUnit(null));
  }, [state.authed, state.baseUrl]);

  // Publish quota for the shell header mini-meter + 28px statusline.
  useEffect(() => {
    if (error && !sub) {
      window.dispatchEvent(
        new CustomEvent("napi:status", { detail: { backendError: error } }),
      );
    } else if (sub) {
      const pct =
        sub.limit > 0 ? Math.min((sub.used / sub.limit) * 100, 100) : 0;
      window.dispatchEvent(
        new CustomEvent("napi:status", {
          detail: { quota: `${Math.round(pct)}%`, backendError: null },
        }),
      );
    }
  }, [sub, error]);

  if (loading && !sub) {
    return (
      <div>
        <div className="page-head">
          <div>
            <h2>Subscription</h2>
            <p className="page-sub">Loading subscription…</p>
          </div>
        </div>
        <div role="status" aria-label="Loading subscription">
          <div className="quota-bar-container" aria-hidden="true">
            <div
              className="skeleton"
              style={{ height: 14, width: "40%", marginBottom: 12 }}
            />
            <div className="skeleton" style={{ height: 8 }} />
            <div
              className="skeleton"
              style={{ height: 12, width: "30%", marginTop: 12 }}
            />
          </div>
          <div className="skeleton-list" aria-hidden="true">
            <div className="skeleton" style={{ height: 44 }} />
            <div className="skeleton" style={{ height: 44 }} />
            <div className="skeleton" style={{ height: 44 }} />
          </div>
        </div>
      </div>
    );
  }

  if (error && !sub) {
    return (
      <div>
        <div className="page-head">
          <div>
            <h2>Subscription</h2>
            <p className="page-sub">Couldn't load subscription</p>
          </div>
          <button className="btn-secondary" onClick={load}>
            Retry
          </button>
        </div>
        <div className="error-banner" role="alert">
          {error}
        </div>
      </div>
    );
  }

  if (!sub) {
    return (
      <div>
        <div className="page-head">
          <div>
            <h2>Subscription</h2>
            <p className="page-sub">No subscription data</p>
          </div>
        </div>
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ○
          </div>
          <h3>No subscription found</h3>
          <p>
            Signed in, but the server returned no subscription for this
            account. Check the endpoint and try again.
          </p>
          <button className="btn-secondary" onClick={load}>
            Check connection
          </button>
        </div>
      </div>
    );
  }

  const pct = sub.limit > 0 ? Math.min((sub.used / sub.limit) * 100, 100) : 0;
  const exhausted = pct >= 100;
  const high = pct > 80;

  const reset = new Date(sub.reset_date);
  const validReset = !Number.isNaN(reset.getTime());
  const resetLabel = validReset ? reset.toLocaleDateString() : sub.reset_date;
  const daysLeft = validReset
    ? Math.max(0, Math.ceil((reset.getTime() - Date.now()) / 86400000))
    : null;

  const billingUrl = `${state.baseUrl.replace(/\/+$/, "")}/billing`;
  const upgrade = /free|trial/i.test(sub.plan);

  const needle = modelsQ.trim().toLowerCase();
  const shown = needle
    ? models.filter((m) =>
        [m.model_name, m.description ?? "", m.tags ?? "", ...(m.supported_endpoint_types ?? [])]
          .join(" ")
          .toLowerCase()
          .includes(needle),
      )
    : models;

  const hasTotal = logsTotal !== null;
  const reqCount = hasTotal ? logsTotal : logs.length;
  const reqLabel = hasTotal
    ? "All requests"
    : logs.length > 0
      ? "Last 50 requests"
      : "Requests";
  const tokenCount = logs.reduce(
    (n, r) => n + (r.prompt_tokens ?? 0) + (r.completion_tokens ?? 0),
    0,
  );
  const tokenLabel =
    !hasTotal || (logsTotal !== null && logsTotal > logs.length)
      ? "Tokens (this page)"
      : "Tokens";

  return (
    <div>
      <div className="mb-4 rounded-xl border border-border p-4">
        <p className="text-xs text-muted-foreground">This week</p>
        {weeklyError ? (
          <p className="mt-1 text-sm text-red-400">{weeklyError}</p>
        ) : weekly ? (
          <>
            <p className="text-2xl font-semibold">
              ${weekly.usedUsd.toFixed(2)}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              {formatCompactCount(weekly.requestCount)} requests ·{" "}
              {formatCompactCount(weekly.tokenUsed)} tokens · last 7 days
              from NAPI quota_data
            </p>
          </>
        ) : (
          <p className="mt-1 text-sm text-muted-foreground">Loading week…</p>
        )}
      </div>
      <div className="mb-4 grid grid-cols-2 gap-3">
        <div className="rounded-xl border border-border p-4">
          <p className="text-xs text-muted-foreground">{reqLabel}</p>
          <p className="text-2xl font-semibold">
            {logsLoading && logs.length === 0 && !logsError
              ? "…"
              : formatCompactCount(reqCount)}
          </p>
        </div>
        <div className="rounded-xl border border-border p-4">
          <p className="text-xs text-muted-foreground">{tokenLabel}</p>
          <p className="text-2xl font-semibold">
            {logsLoading && logs.length === 0 && !logsError
              ? "…"
              : formatCompactCount(tokenCount)}
          </p>
        </div>
      </div>
      <div className="page-head">
        <div>
          <h2>Subscription</h2>
          <p className="page-sub">
            {sub.plan} · ${sub.used.toFixed(2)} / ${sub.limit.toFixed(2)} ·{" "}
            {Math.round(pct)}% used
          </p>
        </div>
        <a
          className="btn-primary"
          href={billingUrl}
          target="_blank"
          rel="noreferrer"
        >
          {upgrade ? "Upgrade" : "Manage billing"}
        </a>
      </div>

      <div className="quota-bar-container">
        <div className="quota-label">
          <span>
            Plan: <strong>{sub.plan}</strong>
          </span>
          <span>
            ${sub.used.toFixed(2)} / ${sub.limit.toFixed(2)}
          </span>
        </div>
        <div
          className="quota-bar"
          role="progressbar"
          aria-label="Quota used"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(pct)}
        >
          <div
            className={`quota-fill${exhausted ? " is-critical" : high ? " warning" : ""}`}
            style={{ width: `${pct}%` }}
          />
        </div>
        <p className="quota-meta">
          {Math.round(pct)}% used · Resets {resetLabel}
          {daysLeft !== null &&
            ` · ${daysLeft} day${daysLeft === 1 ? "" : "s"} left`}
        </p>
        {high && (
          <p
            className="quota-meta"
            role="status"
            style={{
              color: exhausted
                ? "var(--color-error)"
                : "var(--color-warning)",
            }}
          >
            {exhausted
              ? "Quota exhausted — upgrade to continue."
              : "High usage — over 80% of quota used."}
          </p>
        )}
      </div>

      <div className="mb-6">
        <h3 className="section-heading">Recent requests</h3>
        {logsError ? (
          <div>
            <div className="error-banner" role="alert">
              {logsError}
            </div>
            <button
              className="btn-secondary"
              onClick={loadLogs}
              style={{ marginTop: 12 }}
            >
              Retry history
            </button>
          </div>
        ) : logsLoading && logs.length === 0 ? (
          <div className="skeleton-list" aria-hidden="true">
            <div className="skeleton" style={{ height: 44 }} />
            <div className="skeleton" style={{ height: 44 }} />
            <div className="skeleton" style={{ height: 44 }} />
          </div>
        ) : logs.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon" aria-hidden="true">
              ○
            </div>
            <h3>No requests yet</h3>
            <p>Consume logs from this account will show up here.</p>
          </div>
        ) : (
          <div className="divide-y divide-border rounded-xl border border-border">
            {logs.map((r, i) => {
              const tokens =
                (r.prompt_tokens ?? 0) + (r.completion_tokens ?? 0);
              return (
                <div
                  key={r.id ?? i}
                  className="flex items-center justify-between gap-3 px-3 py-2 text-sm"
                >
                  <div className="min-w-0">
                    <div className="font-mono text-xs truncate">
                      {r.model_name ?? "model"}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {formatLogTime(r.created_at)}
                      {r.token_name ? ` · ${r.token_name}` : ""}
                    </div>
                  </div>
                  <div className="shrink-0 text-right text-xs text-muted-foreground">
                    <div>{tokens.toLocaleString()} tok</div>
                    <div>{formatLogQuota(r.quota, quotaPerUnit)}</div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {models.length > 0 && (
        <>
          <h3 className="section-heading">
            Available Models{" "}
            <span className="lib-count">
              {shown.length} of {models.length}
            </span>
          </h3>
          <input
            className="search-input"
            placeholder="Filter models…"
            aria-label="Filter models"
            value={modelsQ}
            onChange={(e) => setModelsQ(e.target.value)}
            style={{ marginBottom: 12 }}
          />
          {shown.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon" aria-hidden="true">
                ○
              </div>
              <h3>No models match</h3>
              <p>No models match “{modelsQ.trim()}”.</p>
              <button
                className="btn-secondary"
                onClick={() => setModelsQ("")}
              >
                Clear filter
              </button>
            </div>
          ) : (
            <div className="model-list">
              {shown.map((m) => (
                <div className="model-row" key={m.model_name}>
                  <div className="model-row-main">
                    <span className="model-name">{m.model_name}</span>
                    {m.tags && <span className="model-tags">{m.tags}</span>}
                  </div>
                  {m.description && (
                    <div className="model-desc">{m.description}</div>
                  )}
                  {m.supported_endpoint_types &&
                    m.supported_endpoint_types.length > 0 && (
                      <div
                        style={{
                          display: "flex",
                          flexWrap: "wrap",
                          gap: 6,
                          marginTop: 8,
                        }}
                      >
                        {m.supported_endpoint_types.map((t) => (
                          <span className="chip" key={t}>
                            {t}
                          </span>
                        ))}
                      </div>
                    )}
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
