import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Edit3, ExternalLink, Search, Server, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { AppCountBar } from "@/components/common/AppCountBar.tsx";
import { AppToggleGroup } from "@/components/common/AppToggleGroup.tsx";
import { ListItemRow } from "@/components/common/ListItemRow.tsx";
import { ManagementListSearch } from "@/components/common/ManagementListSearch.tsx";
import { ConfirmDialog } from "@/components/ConfirmDialog.tsx";
import { mcpPresets } from "@/config/mcpPresets.ts";
import { mcpApi } from "@/lib/mcpApi.ts";
import {
  isMcpAppId,
  MCP_APP_IDS,
  type McpAppId,
  type McpServer,
} from "@/types/mcp.ts";
import { McpFormModal } from "./McpFormModal.tsx";

function getMcpSearchText(id: string, server: McpServer): string {
  const spec = server.server ?? {};
  const values: unknown[] = [
    id,
    server.id,
    server.name,
    server.description,
    ...(Array.isArray(server.tags) ? server.tags : []),
    spec.type,
    spec.command,
    ...(Array.isArray(spec.args) ? spec.args : []),
    spec.cwd,
    spec.url,
    server.homepage,
    server.docs,
  ];
  return values
    .filter((value): value is string => typeof value === "string")
    .join("\n")
    .toLowerCase();
}

export function UnifiedMcpPanel({
  onCountChange,
}: {
  onCountChange?: (count: number) => void;
}) {
  const [serversMap, setServersMap] = useState<Record<string, McpServer>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [writePending, setWritePending] = useState(false);
  const writeLockRef = useRef(false);
  const [pendingApp, setPendingApp] = useState<McpAppId | null>(null);
  const [confirmDialog, setConfirmDialog] = useState<{
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);

  const refresh = useCallback(async () => {
    const data = await mcpApi.getAllServers();
    setServersMap(data ?? {});
  }, []);

  useEffect(() => {
    refresh()
      .catch((e) => toast.error(String(e)))
      .finally(() => setIsLoading(false));
  }, [refresh]);

  const serverEntries = useMemo(
    () => Object.entries(serversMap),
    [serversMap],
  );

  useEffect(() => {
    onCountChange?.(serverEntries.length);
  }, [onCountChange, serverEntries.length]);

  const normalizedSearchQuery = searchQuery.trim().toLowerCase();
  const filteredServerEntries = useMemo(() => {
    if (!normalizedSearchQuery) return serverEntries;
    return serverEntries.filter(([id, server]) =>
      getMcpSearchText(id, server).includes(normalizedSearchQuery),
    );
  }, [normalizedSearchQuery, serverEntries]);

  const enabledCounts = useMemo(() => {
    const counts: Record<McpAppId, number> = {
      claude: 0,
      cursor: 0,
      codex: 0,
      gemini: 0,
      grokbuild: 0,
      opencode: 0,
      hermes: 0,
    };
    for (const [, server] of serverEntries) {
      for (const app of MCP_APP_IDS) {
        if (server.apps[app]) counts[app]++;
      }
    }
    return counts;
  }, [serverEntries]);

  const interactionBlocked = writePending || isFormOpen || confirmDialog !== null;

  const beginWrite = (allowOpenConfirmation = false) => {
    if (
      writeLockRef.current ||
      isFormOpen ||
      (!allowOpenConfirmation && confirmDialog !== null)
    ) {
      return false;
    }
    writeLockRef.current = true;
    setWritePending(true);
    return true;
  };

  const endWrite = () => {
    writeLockRef.current = false;
    setWritePending(false);
    setPendingApp(null);
  };

  const handleToggleApp = async (
    serverId: string,
    app: McpAppId,
    enabled: boolean,
  ) => {
    if (!isMcpAppId(app) || !beginWrite()) return;
    setPendingApp(app);
    try {
      await mcpApi.toggleApp(serverId, app, enabled);
      await refresh();
    } catch (error) {
      toast.error(String(error));
      await refresh().catch(() => undefined);
    } finally {
      endWrite();
    }
  };

  const handleToggleAll = async (app: McpAppId, enabled: boolean) => {
    if (!beginWrite()) return;
    setPendingApp(app);
    const ids = serverEntries
      .filter(([, server]) => Boolean(server.apps[app]) !== enabled)
      .map(([id]) => id);
    if (ids.length === 0) {
      endWrite();
      return;
    }
    const failed: string[] = [];
    try {
      for (const id of ids) {
        try {
          await mcpApi.toggleApp(id, app, enabled);
        } catch {
          failed.push(id);
        }
      }
      if (failed.length) {
        toast.error(`Failed to toggle ${failed.length} server(s)`);
      }
      await refresh();
    } catch (error) {
      toast.error(String(error));
    } finally {
      endWrite();
    }
  };

  const handleImport = async () => {
    if (!beginWrite()) return;
    try {
      const count = await mcpApi.importFromApps();
      await refresh();
      if (count === 0) {
        toast.success("No new servers found in agent configs");
      } else {
        toast.success(`Imported ${count} server(s)`);
      }
    } catch (error) {
      toast.error(String(error));
      await refresh().catch(() => undefined);
    } finally {
      endWrite();
    }
  };

  const handleAdd = () => {
    if (writeLockRef.current || interactionBlocked) return;
    setEditingId(null);
    setIsFormOpen(true);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="mb-4 flex flex-wrap gap-2">
        <Button variant="mcp" size="sm" onClick={handleAdd} disabled={interactionBlocked}>
          Add server
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void handleImport()}
          disabled={interactionBlocked}
        >
          Import from agents
        </Button>
      </div>

      <AppCountBar
        totalLabel={`${serverEntries.length} servers`}
        counts={enabledCounts}
        totalCount={serverEntries.length}
        onToggleAll={handleToggleAll}
        pendingApp={pendingApp}
        disabled={interactionBlocked}
      />

      <ManagementListSearch
        value={searchQuery}
        onValueChange={setSearchQuery}
        placeholder="Search servers…"
        ariaLabel="Search MCP servers"
        clearLabel="Clear"
      />

      <div className="min-h-0 flex-1 overflow-y-auto pb-8">
        {isLoading ? (
          <div className="py-12 text-center text-muted-foreground">Loading MCP…</div>
        ) : serverEntries.length === 0 ? (
          <div className="py-12 text-center">
            <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
              <Server size={24} className="text-muted-foreground" />
            </div>
            <h3 className="mb-2 text-lg font-medium">No MCP servers</h3>
            <p className="text-sm text-muted-foreground">
              Add one, or import from ~/.claude.json and other agent configs.
            </p>
          </div>
        ) : filteredServerEntries.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
            <Search className="mb-4 h-10 w-10 opacity-40" />
            <p className="text-sm">No servers match that search.</p>
          </div>
        ) : (
          <div className="overflow-hidden rounded-xl border border-border-default">
            {filteredServerEntries.map(([id, server], index) => (
              <UnifiedMcpListItem
                key={id}
                id={id}
                server={server}
                onToggleApp={handleToggleApp}
                onEdit={(editId) => {
                  if (writeLockRef.current || interactionBlocked) return;
                  setEditingId(editId);
                  setIsFormOpen(true);
                }}
                onDelete={(deleteId) => {
                  if (writeLockRef.current || interactionBlocked) return;
                  setConfirmDialog({
                    title: "Delete MCP server",
                    message: `Remove “${deleteId}” from the library and from enabled agent configs?`,
                    onConfirm: async () => {
                      if (!beginWrite(true)) return;
                      try {
                        await mcpApi.delete(deleteId);
                        setConfirmDialog(null);
                        toast.success("Deleted");
                        await refresh();
                      } catch (error) {
                        toast.error(String(error));
                      } finally {
                        endWrite();
                      }
                    },
                  });
                }}
                disabled={interactionBlocked}
                isLast={index === filteredServerEntries.length - 1}
              />
            ))}
          </div>
        )}
      </div>

      {isFormOpen ? (
        <McpFormModal
          editingId={editingId || undefined}
          initialData={
            editingId && serversMap[editingId] ? serversMap[editingId] : undefined
          }
          existingIds={Object.keys(serversMap)}
          onSave={async () => {
            setIsFormOpen(false);
            setEditingId(null);
            await refresh();
          }}
          onClose={() => {
            setIsFormOpen(false);
            setEditingId(null);
          }}
        />
      ) : null}

      {confirmDialog ? (
        <ConfirmDialog
          isOpen
          title={confirmDialog.title}
          message={confirmDialog.message}
          pending={writePending}
          onConfirm={() => void confirmDialog.onConfirm()}
          onCancel={() => setConfirmDialog(null)}
        />
      ) : null}
    </div>
  );
}

function UnifiedMcpListItem({
  id,
  server,
  onToggleApp,
  onEdit,
  onDelete,
  disabled,
  isLast,
}: {
  id: string;
  server: McpServer;
  onToggleApp: (serverId: string, app: McpAppId, enabled: boolean) => void;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  disabled?: boolean;
  isLast?: boolean;
}) {
  const name = server.name || id;
  const description = server.description || "";
  const meta = mcpPresets.find((p) => p.id === id);
  const docsUrl = server.docs || meta?.docs;
  const homepageUrl = server.homepage || meta?.homepage;
  const tags = server.tags || meta?.tags;

  return (
    <ListItemRow isLast={isLast}>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate text-sm font-medium text-foreground">
            {name}
          </span>
          {docsUrl || homepageUrl ? (
            <button
              type="button"
              onClick={() => {
                const url = docsUrl || homepageUrl;
                if (url) window.open(url, "_blank", "noopener,noreferrer");
              }}
              className="flex-shrink-0 text-muted-foreground/60 hover:text-foreground"
              title="Open docs"
            >
              <ExternalLink size={12} />
            </button>
          ) : null}
        </div>
        {description ? (
          <p className="truncate text-xs text-muted-foreground" title={description}>
            {description}
          </p>
        ) : tags && tags.length > 0 ? (
          <p className="truncate text-xs text-muted-foreground/60">
            {tags.join(", ")}
          </p>
        ) : null}
      </div>
      <AppToggleGroup
        apps={server.apps}
        onToggle={(app, enabled) => onToggleApp(id, app, enabled)}
        disabled={disabled}
      />
      <div className="flex flex-shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => onEdit(id)}
          disabled={disabled}
          title="Edit"
        >
          <Edit3 size={14} />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-7 w-7 hover:bg-red-100 hover:text-red-500 dark:hover:bg-red-500/10 dark:hover:text-red-400"
          onClick={() => onDelete(id)}
          disabled={disabled}
          title="Delete"
        >
          <Trash2 size={14} />
        </Button>
      </div>
    </ListItemRow>
  );
}
