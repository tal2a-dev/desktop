import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

  useEffect(() => {
    if (!state.authed) return;
    setLoading(true);
    invoke<SubscriptionInfo>("fetch_subscription", {
      baseUrl: state.baseUrl,
    })
      .then((info) => {
        setSub(info);
        setError(null);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [state.authed, state.baseUrl]);

  // ponytail: the catalog is secondary — a failure here must not blank the quota view.
  useEffect(() => {
    if (!state.authed) return;
    invoke<{ data: PricingModel[] }>("get_pricing", { baseUrl: state.baseUrl })
      .then((res) => setModels(Array.isArray(res?.data) ? res.data : []))
      .catch(() => setModels([]));
  }, [state.authed, state.baseUrl]);

  if (loading)
    return <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>;
  if (error) return <div className="error-banner">{error}</div>;
  if (!sub) return null;

  const pct = sub.limit > 0 ? Math.min((sub.used / sub.limit) * 100, 100) : 0;
  const warning = pct > 80;

  return (
    <div>
      <h2>Subscription</h2>
      <div className="quota-bar-container">
        <div className="quota-label">
          <span>
            Plan: <strong>{sub.plan}</strong>
          </span>
          <span>
            ${sub.used.toFixed(2)} / ${sub.limit.toFixed(2)}
          </span>
        </div>
        <div className="quota-track">
          <div
            className={`quota-fill ${warning ? "warning" : ""}`}
            style={{ width: `${pct}%` }}
          />
        </div>
        <p
          style={{
            marginTop: 8,
            fontSize: "0.8rem",
            color: "var(--color-text-muted)",
          }}
        >
          Resets: {sub.reset_date}
        </p>
      </div>

      {models.length > 0 && (
        <>
          <h3 className="section-heading">Available Models</h3>
          <div className="model-list">
            {models.map((m) => (
              <div className="model-row" key={m.model_name}>
                <div className="model-row-main">
                  <span className="model-name">{m.model_name}</span>
                  {m.tags && <span className="model-tags">{m.tags}</span>}
                </div>
                {m.description && (
                  <div className="model-desc">{m.description}</div>
                )}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
