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

export function SubscriptionPage() {
  const { state } = useAuth();
  const [sub, setSub] = useState<SubscriptionInfo | null>(null);
  const [models, setModels] = useState<PricingModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modelsQ, setModelsQ] = useState("");

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
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
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

  return (
    <div>
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
