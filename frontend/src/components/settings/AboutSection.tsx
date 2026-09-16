import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  ArrowUpCircle,
  CheckCircle2,
  Download,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { toast } from "sonner";
import { AgentIcon } from "@/components/AgentIcon.tsx";
import { Button } from "@/components/ui/button.tsx";
import { invoke } from "@/lib/tauri.ts";
import {
  TOOL_IDS,
  TOOL_LABELS,
  isUpdateAvailable,
  normalizeToolVersion,
  type ToolId,
  type ToolVersion,
} from "./types.ts";

interface AboutSectionProps {
  tools: ToolVersion[];
  loading: boolean;
  onRefresh: () => Promise<void>;
}

export function AboutSection({ tools, loading, onRefresh }: AboutSectionProps) {
  const [busy, setBusy] = useState<string | null>(null);

  const byName = useMemo(() => {
    const map = new Map<string, ToolVersion>();
    for (const tool of tools) map.set(tool.name, tool);
    return map;
  }, [tools]);

  const updatable = useMemo(
    () =>
      TOOL_IDS.filter((id) => {
        const tool = byName.get(id);
        return (
          Boolean(tool?.version) &&
          !tool?.installed_but_broken &&
          isUpdateAvailable(tool?.version, tool?.latest_version)
        );
      }),
    [byName],
  );

  const runAction = async (ids: ToolId[], action: "install" | "update") => {
    const cmd = action === "install" ? "install_agent" : "update_agent";
    setBusy(ids.length > 1 ? "__all__" : ids[0] ?? null);
    let failed = 0;
    try {
      for (const id of ids) {
        try {
          const msg = await invoke<string>(cmd, { agentId: id });
          if (msg) toast.success(msg);
        } catch (e) {
          failed += 1;
          toast.error(`${TOOL_LABELS[id]}: ${String(e)}`);
        }
      }
      if (failed === 0 && ids.length > 1) {
        toast.success(
          `${action === "install" ? "Install" : "Update"} completed for ${ids.length} tool(s)`,
        );
      }
      await onRefresh();
    } finally {
      setBusy(null);
    }
  };

  const anyBusy = busy !== null;

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 px-1 sm:flex-row sm:items-center sm:justify-between">
        <h3 className="text-sm font-medium">Local environment check</h3>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            className="h-7 gap-1.5 text-xs"
            onClick={() => void onRefresh()}
            disabled={loading || anyBusy}
          >
            <RefreshCw
              className={loading ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"}
            />
            {loading ? "Refreshing" : "Refresh"}
          </Button>
          <Button
            size="sm"
            className="h-7 gap-1.5 text-xs"
            onClick={() => void runAction(updatable, "update")}
            disabled={loading || anyBusy || updatable.length === 0}
          >
            {busy === "__all__" ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <ArrowUpCircle className="h-3.5 w-3.5" />
            )}
            Update All ({updatable.length})
          </Button>
        </div>
      </div>

      <div className="grid gap-3 px-1 sm:grid-cols-2 xl:grid-cols-3">
        {TOOL_IDS.map((id) => {
          const tool = byName.get(id);
          const isToolLoading = loading && !tool;
          const outdated = isUpdateAvailable(tool?.version, tool?.latest_version);
          const broken = Boolean(tool?.installed_but_broken);
          const action: "install" | "update" | null =
            isToolLoading || broken
              ? null
              : !tool?.version
                ? "install"
                : outdated
                  ? "update"
                  : null;
          const running = busy === id || busy === "__all__";
          const title = tool?.version || tool?.error || "Unknown";

          return (
            <div
              key={id}
              className="flex min-h-[150px] flex-col gap-3 rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm transition-colors hover:border-primary/30"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="flex min-w-0 items-center gap-2">
                  <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-background/80">
                    <AgentIcon id={id} name={TOOL_LABELS[id]} size={16} />
                  </span>
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">
                      {TOOL_LABELS[id]}
                    </div>
                  </div>
                </div>
                {isToolLoading ? (
                  <Loader2 className="mt-1 h-4 w-4 animate-spin text-muted-foreground" />
                ) : tool?.version ? (
                  outdated ? (
                    <span className="mt-1 shrink-0 rounded-full border border-yellow-500/20 bg-yellow-500/10 px-1.5 py-0.5 text-[10px] text-yellow-600 dark:text-yellow-400">
                      Update
                    </span>
                  ) : (
                    <CheckCircle2 className="mt-1 h-4 w-4 shrink-0 text-green-500" />
                  )
                ) : (
                  <AlertCircle className="mt-1 h-4 w-4 shrink-0 text-yellow-500" />
                )}
              </div>

              <div className="space-y-1.5 text-xs">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-muted-foreground">Current Version</span>
                  <span
                    className="min-w-0 truncate font-mono text-foreground"
                    title={title}
                  >
                    {isToolLoading
                      ? "Loading"
                      : tool?.version
                        ? tool.version
                        : broken
                          ? "Installed · can't run"
                          : "Not installed"}
                  </span>
                </div>
                <div className="flex items-center justify-between gap-3">
                  <span className="text-muted-foreground">Latest Version</span>
                  <span className="min-w-0 truncate font-mono text-foreground">
                    {isToolLoading
                      ? "Loading"
                      : tool?.latest_version || "Unknown"}
                  </span>
                </div>
                {!isToolLoading && !tool?.version && tool?.error ? (
                  <div className="truncate text-[11px] text-muted-foreground">
                    {tool.error}
                  </div>
                ) : null}
              </div>

              <div className="mt-auto flex items-center justify-end">
                {isToolLoading ? (
                  <span className="text-xs text-muted-foreground">Loading</span>
                ) : broken ? (
                  <span className="text-xs text-yellow-600 dark:text-yellow-400">
                    Check environment
                  </span>
                ) : action ? (
                  <Button
                    size="sm"
                    variant={action === "install" ? "outline" : "default"}
                    className="h-7 gap-1.5 text-xs"
                    onClick={() => void runAction([id], action)}
                    disabled={anyBusy}
                  >
                    {running ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : action === "install" ? (
                      <Download className="h-3.5 w-3.5" />
                    ) : (
                      <ArrowUpCircle className="h-3.5 w-3.5" />
                    )}
                    {action === "install" ? "Install" : "Update"}
                  </Button>
                ) : (
                  <span className="text-xs text-muted-foreground">Ready</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function useToolVersions() {
  const [tools, setTools] = useState<ToolVersion[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const raw = await invoke<unknown[]>("get_tool_versions", {
        tools: [...TOOL_IDS],
      });
      setTools((Array.isArray(raw) ? raw : []).map(normalizeToolVersion));
    } catch (e) {
      toast.error(String(e));
      setTools([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { tools, loading, refresh };
}
