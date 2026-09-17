import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { cn } from "@/lib/utils.ts";
import { invoke } from "@/lib/tauri.ts";
import { useAuth } from "@/lib/auth.tsx";

export interface ApiKeyInfo {
  id: number;
  name: string;
  status: number;
  remainQuota: number;
  usedQuota: number;
  unlimitedQuota: boolean;
  expiredTime: number;
  keyMasked: string;
  selected: boolean;
}

const STATUS: Record<number, { label: string; className: string }> = {
  1: { label: "Enabled", className: "text-emerald-500" },
  2: { label: "Disabled", className: "text-muted-foreground" },
  3: { label: "Expired", className: "text-amber-500" },
  4: { label: "Exhausted", className: "text-red-500" },
};

function normalizeKey(raw: Record<string, unknown>): ApiKeyInfo {
  const n = (v: unknown) =>
    typeof v === "number" && Number.isFinite(v) ? v : 0;
  return {
    id: n(raw.id),
    name: typeof raw.name === "string" ? raw.name : "unnamed",
    status: n(raw.status),
    remainQuota: n(raw.remainQuota ?? raw.remain_quota),
    usedQuota: n(raw.usedQuota ?? raw.used_quota),
    unlimitedQuota: Boolean(raw.unlimitedQuota ?? raw.unlimited_quota),
    expiredTime: n(raw.expiredTime ?? raw.expired_time),
    keyMasked: String(raw.keyMasked ?? raw.key_masked ?? raw.key ?? ""),
    selected: Boolean(raw.selected),
  };
}

export function ApiKeyPicker() {
  const { state } = useAuth();
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    if (!state.authed) return;
    setLoading(true);
    setError(null);
    invoke<unknown>("list_api_keys", { baseUrl: state.baseUrl })
      .then((res) => {
        const list = Array.isArray(res) ? res : [];
        setKeys(list.map((row) => normalizeKey(row as Record<string, unknown>)));
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load();
  }, [state.authed, state.baseUrl]);

  const pick = async (id: number) => {
    setBusyId(id);
    try {
      const picked = normalizeKey(
        (await invoke<Record<string, unknown>>("select_api_key", {
          baseUrl: state.baseUrl,
          tokenId: id,
        })) as Record<string, unknown>,
      );
      setKeys((prev) =>
        prev.map((k) => ({ ...k, selected: k.id === picked.id })),
      );
      toast.success(
        `${picked.name} selected and wrote to all agents`,
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyId(null);
    }
  };

  const applyAgents = async () => {
    setBusyId(-1);
    try {
      const lines = await invoke<string[]>("auto_configure_all", {
        baseUrl: state.baseUrl,
      });
      toast.success(lines?.join("\n") || "Agents updated");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyId(null);
    }
  };

  return (
    <section className="space-y-4 rounded-xl border border-border p-4 glass-card">
      <header className="space-y-1">
        <h2 className="text-sm font-medium">API key</h2>
        <p className="text-xs text-muted-foreground">
          Keys live on NAPI (`/api/token`). Listing is masked; picking a key
          reveals the secret and stores it in the OS keychain for Overwrite.
        </p>
      </header>

      {loading ? (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading keys from NAPI…
        </p>
      ) : error ? (
        <div className="space-y-2">
          <p className="text-sm text-red-400">{error}</p>
          <Button size="sm" variant="outline" onClick={load}>
            Retry
          </Button>
        </div>
      ) : keys.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No API keys on this account. Sign-in setup will mint{" "}
          <span className="font-mono">napi-desktop</span>.
        </p>
      ) : (
        <ul className="space-y-2">
          {keys.map((k) => {
            const st = STATUS[k.status] ?? STATUS[2];
            return (
              <li key={k.id}>
                <button
                  type="button"
                  disabled={busyId !== null || k.status !== 1}
                  onClick={() => void pick(k.id)}
                  className={cn(
                    "flex w-full items-start justify-between gap-3 rounded-lg border px-3 py-2 text-left text-sm transition-colors",
                    k.selected
                      ? "border-blue-500/50 bg-blue-500/10"
                      : "border-border hover:bg-muted/40",
                    k.status !== 1 && "opacity-60",
                  )}
                >
                  <span className="min-w-0">
                    <span className="block truncate font-medium">{k.name}</span>
                    <span className="block font-mono text-[11px] text-muted-foreground">
                      sk-{k.keyMasked}
                    </span>
                    <span className="block text-[11px] text-muted-foreground">
                      {k.unlimitedQuota
                        ? "Unlimited quota"
                        : `${k.usedQuota.toLocaleString()} used · ${k.remainQuota.toLocaleString()} left`}
                    </span>
                  </span>
                  <span className="flex shrink-0 flex-col items-end gap-1">
                    {busyId === k.id ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : k.selected ? (
                      <span className="text-[11px] font-medium text-blue-500">
                        Selected
                      </span>
                    ) : null}
                    <span className={cn("text-[11px]", st.className)}>
                      {st.label}
                    </span>
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}

      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="outline" onClick={load} disabled={loading}>
          Refresh
        </Button>
        <Button
          size="sm"
          onClick={() => void applyAgents()}
          disabled={busyId !== null || !keys.some((k) => k.selected)}
        >
          {busyId === -1 ? (
            <span className="inline-flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              Writing agents
            </span>
          ) : (
            "Apply to agents"
          )}
        </Button>
      </div>
    </section>
  );
}
