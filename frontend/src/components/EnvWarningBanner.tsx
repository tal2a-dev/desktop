import { useState } from "react";
import { AlertTriangle, ChevronDown, ChevronUp, Trash2, X } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { invoke } from "@/lib/tauri.ts";

export interface EnvConflict {
  varName: string;
  varValue: string;
  sourceType: string;
  sourcePath: string;
}

export function EnvWarningBanner({
  conflicts,
  onDismiss,
  onDeleted,
}: {
  conflicts: EnvConflict[];
  onDismiss: () => void;
  onDeleted: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  if (conflicts.length === 0) return null;

  const keyOf = (c: EnvConflict) => `${c.varName}:${c.sourcePath}`;
  const toggleAll = () => {
    if (selected.size === conflicts.length) setSelected(new Set());
    else setSelected(new Set(conflicts.map(keyOf)));
  };

  const deleteSelected = async () => {
    const list = conflicts.filter((c) => selected.has(keyOf(c)));
    if (list.length === 0) {
      toast.warning("Select variables to delete");
      return;
    }
    try {
      const backup = await invoke<{ backupPath: string }>("delete_env_vars", {
        conflicts: list,
      });
      toast.success(`Removed ${list.length} variable(s)`, {
        description: backup?.backupPath
          ? `Backup: ${backup.backupPath}`
          : undefined,
      });
      setSelected(new Set());
      onDeleted();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="border-b border-amber-500/40 bg-amber-950/80 px-4 py-3 text-amber-100">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-2">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <div>
            <p className="text-sm font-semibold">
              Environment Variable Conflicts Detected
            </p>
            <p className="text-xs opacity-90">
              Found {conflicts.length} environment variable
              {conflicts.length === 1 ? "" : "s"} that may override your
              configuration
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-8 text-amber-100 hover:bg-amber-900/50"
            onClick={() => setExpanded((v) => !v)}
          >
            {expanded ? "Collapse" : "View Details"}
            {expanded ? (
              <ChevronUp className="ml-1 h-3.5 w-3.5" />
            ) : (
              <ChevronDown className="ml-1 h-3.5 w-3.5" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-amber-100"
            onClick={onDismiss}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
      {expanded && (
        <div className="mt-3 space-y-2">
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={
                conflicts.length > 0 && selected.size === conflicts.length
              }
              onChange={toggleAll}
            />
            Select All
          </label>
          {conflicts.map((c) => {
            const k = keyOf(c);
            const processEnv = c.sourcePath === "Process Environment";
            return (
              <label
                key={k}
                className="flex items-start gap-2 rounded-lg border border-amber-500/20 bg-black/20 p-3 text-xs"
              >
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={selected.has(k)}
                  onChange={() => {
                    const next = new Set(selected);
                    if (next.has(k)) next.delete(k);
                    else next.add(k);
                    setSelected(next);
                  }}
                />
                <span className="min-w-0">
                  <span className="block font-mono font-medium">{c.varName}</span>
                  <span className="block truncate opacity-80">
                    Value: {c.varValue}
                  </span>
                  <span className="block opacity-70">Source: {c.sourcePath}</span>
                  {processEnv && (
                    <span className="mt-1 block text-amber-200/80">
                      Delete clears it for this app and agents launched from
                      here, and strips matching export lines from your shell
                      profiles.
                    </span>
                  )}
                </span>
              </label>
            );
          })}
          <div className="flex justify-end gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSelected(new Set())}
            >
              Clear Selection
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => void deleteSelected()}
            >
              <Trash2 className="mr-1 h-3.5 w-3.5" />
              Delete Selected ({selected.size})
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
