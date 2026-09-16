import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import {
  AlertCircle,
  ChevronDown,
  ChevronUp,
  Plus,
  Save,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button.tsx";
import { Input } from "@/components/ui/input.tsx";
import { mcpPresets } from "@/config/mcpPresets.ts";
import { MCP_APP_VISUAL } from "@/config/mcpApps.tsx";
import { mcpApi } from "@/lib/mcpApi.ts";
import {
  defaultEnabledApps,
  EMPTY_APPS,
  MCP_APP_IDS,
  type McpApps,
  type McpServer,
  type McpServerSpec,
} from "@/types/mcp.ts";
import { McpWizardModal } from "./McpWizardModal.tsx";
import { parseSmartMcpJson, useMcpValidation } from "./useMcpValidation.ts";

export function McpFormModal({
  editingId,
  initialData,
  existingIds = [],
  onSave,
  onClose,
}: {
  editingId?: string;
  initialData?: McpServer;
  existingIds?: string[];
  onSave: () => Promise<void>;
  onClose: () => void;
}) {
  const { validateJsonConfig } = useMcpValidation();
  const isEditing = !!editingId;
  const [formId, setFormId] = useState(
    () => editingId || initialData?.id || "",
  );
  const [formName, setFormName] = useState(initialData?.name || "");
  const [formDescription, setFormDescription] = useState(
    initialData?.description || "",
  );
  const [formHomepage, setFormHomepage] = useState(initialData?.homepage || "");
  const [formDocs, setFormDocs] = useState(initialData?.docs || "");
  const [formTags, setFormTags] = useState(initialData?.tags?.join(", ") || "");
  const [enabledApps, setEnabledApps] = useState<McpApps>(() => {
    if (initialData?.apps) {
      return { ...EMPTY_APPS, ...initialData.apps };
    }
    return defaultEnabledApps();
  });
  const [formConfig, setFormConfig] = useState(() =>
    initialData?.server ? JSON.stringify(initialData.server, null, 2) : "",
  );
  const [configError, setConfigError] = useState("");
  const [idError, setIdError] = useState("");
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const [isWizardOpen, setIsWizardOpen] = useState(false);
  const [showMetadata, setShowMetadata] = useState(
    !!(
      initialData?.description ||
      initialData?.tags?.length ||
      initialData?.homepage ||
      initialData?.docs
    ),
  );
  const [selectedPreset, setSelectedPreset] = useState<number | null>(
    isEditing ? null : -1,
  );

  const wizardInitialSpec = useMemo(() => {
    if (!formConfig.trim()) return initialData?.server;
    try {
      return parseSmartMcpJson(formConfig).config;
    } catch {
      return initialData?.server;
    }
  }, [formConfig, initialData]);

  const ensureUniqueId = (base: string): string => {
    let candidate = base.trim() || "mcp-server";
    if (!existingIds.includes(candidate)) return candidate;
    let i = 1;
    while (existingIds.includes(`${candidate}-${i}`)) i++;
    return `${candidate}-${i}`;
  };

  const applyPreset = (index: number) => {
    const preset = mcpPresets[index];
    if (!preset) return;
    const id = ensureUniqueId(preset.id);
    setFormId(id);
    setFormName(preset.name);
    setFormDescription(preset.description);
    setFormHomepage(preset.homepage || "");
    setFormDocs(preset.docs || "");
    setFormTags(preset.tags?.join(", ") || "");
    const json = JSON.stringify(preset.server, null, 2);
    setFormConfig(json);
    setConfigError(validateJsonConfig(json));
    setSelectedPreset(index);
  };

  const handleIdChange = (value: string) => {
    setFormId(value);
    if (!isEditing) {
      setIdError(
        existingIds.includes(value.trim()) ? "This id already exists" : "",
      );
    }
  };

  const handleConfigChange = (value: string) => {
    setFormConfig(value);
    try {
      const result = parseSmartMcpJson(value);
      const err = validateJsonConfig(JSON.stringify(result.config));
      setConfigError(err);
      if (result.id && !formId.trim() && !isEditing) {
        const uniqueId = ensureUniqueId(result.id);
        setFormId(uniqueId);
        if (!formName.trim()) setFormName(result.id);
      }
    } catch (err) {
      setConfigError(err instanceof Error ? err.message : "Invalid JSON");
    }
  };

  const handleSubmit = async () => {
    if (savingRef.current) return;
    const trimmedId = formId.trim();
    if (!trimmedId) {
      toast.error("Server id is required");
      return;
    }
    if (!isEditing && existingIds.includes(trimmedId)) {
      setIdError("This id already exists");
      return;
    }
    let serverSpec: McpServerSpec;
    if (!formConfig.trim()) {
      serverSpec = { type: "stdio", command: "", args: [] };
    } else {
      try {
        serverSpec = parseSmartMcpJson(formConfig).config;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Invalid JSON";
        setConfigError(msg);
        toast.error(msg);
        return;
      }
    }
    if ((serverSpec.type ?? "stdio") === "stdio" && !serverSpec.command?.trim()) {
      toast.error("Command is required");
      return;
    }
    if (
      (serverSpec.type === "http" || serverSpec.type === "sse") &&
      !serverSpec.url?.trim()
    ) {
      toast.error("URL is required");
      return;
    }

    savingRef.current = true;
    setSaving(true);
    try {
      const tags = formTags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);
      const entry: McpServer = {
        ...(initialData ?? { id: trimmedId, name: trimmedId, server: serverSpec, apps: enabledApps }),
        id: trimmedId,
        name: (formName || trimmedId).trim() || trimmedId,
        server: serverSpec,
        apps: enabledApps,
      };
      entry.description = formDescription.trim() || undefined;
      entry.homepage = formHomepage.trim() || undefined;
      entry.docs = formDocs.trim() || undefined;
      entry.tags = tags.length ? tags : undefined;
      await mcpApi.upsert(entry);
      toast.success("Saved");
      await onSave();
    } catch (error) {
      toast.error(String(error));
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <>
      <div className="fixed inset-0 z-50 flex flex-col bg-background text-foreground">
        <header className="flex flex-shrink-0 items-center justify-between border-b border-border-default px-6 py-4">
          <h2 className="text-lg font-semibold">
            {isEditing ? "Edit MCP server" : "Add MCP server"}
          </h2>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => {
              if (!savingRef.current) onClose();
            }}
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </Button>
        </header>
        <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 py-6">
          <div className="glass space-y-6 rounded-xl border border-white/10 p-6">
            {!isEditing && (
              <div>
                <label className="mb-3 block text-sm font-medium">Presets</label>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => {
                      setSelectedPreset(-1);
                      setFormId("");
                      setFormName("");
                      setFormDescription("");
                      setFormHomepage("");
                      setFormDocs("");
                      setFormTags("");
                      setFormConfig("");
                      setConfigError("");
                    }}
                    className={`inline-flex items-center rounded-lg px-4 py-2 text-sm font-medium ${
                      selectedPreset === -1
                        ? "bg-emerald-500 text-white"
                        : "bg-accent text-muted-foreground hover:bg-accent/80"
                    }`}
                  >
                    Custom
                  </button>
                  {mcpPresets.map((preset, idx) => (
                    <button
                      key={preset.id}
                      type="button"
                      onClick={() => applyPreset(idx)}
                      title={preset.description}
                      className={`inline-flex items-center rounded-lg px-4 py-2 text-sm font-medium ${
                        selectedPreset === idx
                          ? "bg-emerald-500 text-white"
                          : "bg-accent text-muted-foreground hover:bg-accent/80"
                      }`}
                    >
                      {preset.id}
                    </button>
                  ))}
                </div>
              </div>
            )}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <label className="text-sm font-medium">
                  Id <span className="text-red-500">*</span>
                </label>
                {!isEditing && idError ? (
                  <span className="text-xs text-red-500">{idError}</span>
                ) : null}
              </div>
              <Input
                value={formId}
                onChange={(e) => handleIdChange(e.target.value)}
                disabled={isEditing}
                placeholder="filesystem"
              />
            </div>
            <div>
              <label className="mb-2 block text-sm font-medium">Name</label>
              <Input
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="Display name"
              />
            </div>
            <div>
              <label className="mb-3 block text-sm font-medium">
                Enable on
              </label>
              <div className="flex flex-wrap gap-4">
                {MCP_APP_IDS.map((app) => (
                  <label
                    key={app}
                    className="inline-flex cursor-pointer select-none items-center gap-2 text-sm"
                  >
                    <input
                      type="checkbox"
                      checked={enabledApps[app]}
                      onChange={(e) =>
                        setEnabledApps({ ...enabledApps, [app]: e.target.checked })
                      }
                      className="h-4 w-4 accent-blue-500"
                    />
                    {MCP_APP_VISUAL[app].label}
                  </label>
                ))}
              </div>
            </div>
            <button
              type="button"
              onClick={() => setShowMetadata((v) => !v)}
              className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground"
            >
              {showMetadata ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
              Additional info
            </button>
            {showMetadata ? (
              <div className="space-y-4">
                <Input
                  value={formDescription}
                  onChange={(e) => setFormDescription(e.target.value)}
                  placeholder="Description"
                />
                <Input
                  value={formTags}
                  onChange={(e) => setFormTags(e.target.value)}
                  placeholder="tags, comma, separated"
                />
                <Input
                  value={formHomepage}
                  onChange={(e) => setFormHomepage(e.target.value)}
                  placeholder="Homepage URL"
                />
                <Input
                  value={formDocs}
                  onChange={(e) => setFormDocs(e.target.value)}
                  placeholder="Docs URL"
                />
              </div>
            ) : null}
          </div>
          <div className="glass flex min-h-[240px] flex-1 flex-col rounded-xl border border-white/10 p-6">
            <div className="mb-4 flex items-center justify-between">
              <label className="text-sm font-medium">JSON config</label>
              {(isEditing || selectedPreset === -1) && (
                <button
                  type="button"
                  onClick={() => setIsWizardOpen(true)}
                  className="text-sm text-blue-500 hover:text-blue-600"
                >
                  Use wizard
                </button>
              )}
            </div>
            <textarea
              value={formConfig}
              onChange={(e) => handleConfigChange(e.target.value)}
              placeholder='{"type":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-filesystem"]}'
              className="min-h-[180px] flex-1 resize-y rounded-md border border-border-default bg-background p-3 font-mono text-sm"
            />
            {configError ? (
              <div className="mt-2 flex items-center gap-2 text-sm text-red-500">
                <AlertCircle size={16} />
                <span>{configError}</span>
              </div>
            ) : null}
          </div>
        </div>
        <footer className="flex flex-shrink-0 justify-end border-t border-border-default px-6 py-4">
          <Button
            onClick={() => void handleSubmit()}
            disabled={saving || (!isEditing && !!idError)}
          >
            {isEditing ? <Save size={16} /> : <Plus size={16} />}
            {saving ? "Saving…" : isEditing ? "Save" : "Add"}
          </Button>
        </footer>
      </div>
      <McpWizardModal
        isOpen={isWizardOpen}
        onClose={() => setIsWizardOpen(false)}
        onApply={(title, json) => {
          setFormId(title);
          if (!formName.trim()) setFormName(title);
          setFormConfig(json);
          setConfigError(validateJsonConfig(json));
        }}
        initialTitle={formId}
        initialServer={wizardInitialSpec}
      />
    </>
  );
}
