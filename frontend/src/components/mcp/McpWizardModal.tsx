import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { Input } from "@/components/ui/input.tsx";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog.tsx";
import type { McpServerSpec } from "@/types/mcp.ts";

function parseEnvText(text: string): Record<string, string> {
  const env: Record<string, string> = {};
  for (const line of text.split("\n").map((l) => l.trim()).filter(Boolean)) {
    const idx = line.indexOf("=");
    if (idx > 0) env[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }
  return env;
}

function parseHeadersText(text: string): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const line of text.split("\n").map((l) => l.trim()).filter(Boolean)) {
    const colonIdx = line.indexOf(":");
    const equalIdx = line.indexOf("=");
    let idx = -1;
    if (colonIdx > 0 && (equalIdx === -1 || colonIdx < equalIdx)) idx = colonIdx;
    else if (equalIdx > 0) idx = equalIdx;
    if (idx > 0) headers[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }
  return headers;
}

export function McpWizardModal({
  isOpen,
  onClose,
  onApply,
  initialTitle,
  initialServer,
}: {
  isOpen: boolean;
  onClose: () => void;
  onApply: (title: string, json: string) => void;
  initialTitle?: string;
  initialServer?: McpServerSpec;
}) {
  const [wizardType, setWizardType] = useState<"stdio" | "http" | "sse">(
    "stdio",
  );
  const [wizardTitle, setWizardTitle] = useState("");
  const [wizardCommand, setWizardCommand] = useState("");
  const [wizardArgs, setWizardArgs] = useState("");
  const [wizardEnv, setWizardEnv] = useState("");
  const [wizardUrl, setWizardUrl] = useState("");
  const [wizardHeaders, setWizardHeaders] = useState("");

  const generatePreview = (): string => {
    const config: McpServerSpec = { type: wizardType };
    if (wizardType === "stdio") {
      config.command = wizardCommand.trim();
      if (wizardArgs.trim()) {
        config.args = wizardArgs.split("\n").map((s) => s.trim()).filter(Boolean);
      }
      if (wizardEnv.trim()) {
        const env = parseEnvText(wizardEnv);
        if (Object.keys(env).length > 0) config.env = env;
      }
    } else {
      config.url = wizardUrl.trim();
      if (wizardHeaders.trim()) {
        const headers = parseHeadersText(wizardHeaders);
        if (Object.keys(headers).length > 0) config.headers = headers;
      }
    }
    return JSON.stringify(config, null, 2);
  };

  const handleClose = () => {
    setWizardType("stdio");
    setWizardTitle("");
    setWizardCommand("");
    setWizardArgs("");
    setWizardEnv("");
    setWizardUrl("");
    setWizardHeaders("");
    onClose();
  };

  const handleApply = () => {
    if (!wizardTitle.trim()) {
      toast.error("Server id is required");
      return;
    }
    if (wizardType === "stdio" && !wizardCommand.trim()) {
      toast.error("Command is required");
      return;
    }
    if ((wizardType === "http" || wizardType === "sse") && !wizardUrl.trim()) {
      toast.error("URL is required");
      return;
    }
    onApply(wizardTitle.trim(), generatePreview());
    handleClose();
  };

  useEffect(() => {
    if (!isOpen) return;
    setWizardTitle(initialTitle ?? "");
    const resolvedType =
      initialServer?.type ?? (initialServer?.url ? "http" : "stdio");
    setWizardType(resolvedType);
    if (resolvedType === "http" || resolvedType === "sse") {
      setWizardUrl(initialServer?.url ?? "");
      const headers = initialServer?.headers;
      setWizardHeaders(
        headers
          ? Object.entries(headers)
              .map(([k, v]) => `${k}: ${v ?? ""}`)
              .join("\n")
          : "",
      );
      setWizardCommand("");
      setWizardArgs("");
      setWizardEnv("");
      return;
    }
    setWizardCommand(initialServer?.command ?? "");
    setWizardArgs(
      Array.isArray(initialServer?.args) ? initialServer.args.join("\n") : "",
    );
    const env = initialServer?.env;
    setWizardEnv(
      env
        ? Object.entries(env)
            .map(([k, v]) => `${k}=${v ?? ""}`)
            .join("\n")
        : "",
    );
    setWizardUrl("");
    setWizardHeaders("");
  }, [isOpen, initialTitle, initialServer]);

  const preview = generatePreview();

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="max-h-[90vh] max-w-2xl flex flex-col" zIndex="alert">
        <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
          <DialogTitle className="text-lg font-semibold">
            MCP configuration wizard
          </DialogTitle>
        </DialogHeader>
        <div className="flex-1 space-y-4 overflow-y-auto px-6 py-4">
          <div className="rounded-lg border border-border-default bg-gray-100/50 p-3 dark:bg-gray-800/50">
            <p className="text-sm text-muted-foreground">
              Fill the fields to generate a single-server JSON spec. Cmd/Ctrl+Enter
              applies it.
            </p>
          </div>
          <div>
            <label className="mb-2 block text-sm font-medium">
              Type <span className="text-red-500">*</span>
            </label>
            <div className="flex gap-4">
              {(["stdio", "http", "sse"] as const).map((t) => (
                <label key={t} className="inline-flex cursor-pointer items-center gap-2">
                  <input
                    type="radio"
                    value={t}
                    checked={wizardType === t}
                    onChange={() => setWizardType(t)}
                    className="h-4 w-4 accent-blue-500"
                  />
                  <span className="text-sm uppercase">{t}</span>
                </label>
              ))}
            </div>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium">
              Id <span className="text-red-500">*</span>
            </label>
            <Input
              value={wizardTitle}
              onChange={(e) => setWizardTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && e.metaKey) {
                  e.preventDefault();
                  handleApply();
                }
              }}
              placeholder="filesystem"
              className="font-mono"
            />
          </div>
          {wizardType === "stdio" ? (
            <>
              <div>
                <label className="mb-1 block text-sm font-medium">
                  Command <span className="text-red-500">*</span>
                </label>
                <Input
                  value={wizardCommand}
                  onChange={(e) => setWizardCommand(e.target.value)}
                  placeholder="npx"
                  className="font-mono"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">
                  Args (one per line)
                </label>
                <textarea
                  value={wizardArgs}
                  onChange={(e) => setWizardArgs(e.target.value)}
                  placeholder={"-y\n@modelcontextprotocol/server-filesystem"}
                  rows={3}
                  className="w-full resize-y rounded-md border border-border-default bg-background px-3 py-2 font-mono text-sm"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">
                  Env (KEY=value)
                </label>
                <textarea
                  value={wizardEnv}
                  onChange={(e) => setWizardEnv(e.target.value)}
                  placeholder="API_KEY=…"
                  rows={3}
                  className="w-full resize-y rounded-md border border-border-default bg-background px-3 py-2 font-mono text-sm"
                />
              </div>
            </>
          ) : (
            <>
              <div>
                <label className="mb-1 block text-sm font-medium">
                  URL <span className="text-red-500">*</span>
                </label>
                <Input
                  value={wizardUrl}
                  onChange={(e) => setWizardUrl(e.target.value)}
                  placeholder="https://example.com/mcp"
                  className="font-mono"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium">
                  Headers (KEY: value)
                </label>
                <textarea
                  value={wizardHeaders}
                  onChange={(e) => setWizardHeaders(e.target.value)}
                  placeholder="Authorization: Bearer …"
                  rows={3}
                  className="w-full resize-y rounded-md border border-border-default bg-background px-3 py-2 font-mono text-sm"
                />
              </div>
            </>
          )}
          {(wizardCommand || wizardArgs || wizardEnv || wizardUrl || wizardHeaders) && (
            <div className="space-y-2 border-t border-border-default pt-4">
              <h3 className="text-sm font-medium">Preview</h3>
              <pre className="overflow-x-auto rounded-lg bg-gray-100 p-3 font-mono text-xs dark:bg-gray-800">
                {preview}
              </pre>
            </div>
          )}
        </div>
        <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
          <Button variant="outline" onClick={handleClose}>
            Cancel
          </Button>
          <Button onClick={handleApply}>Apply</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
