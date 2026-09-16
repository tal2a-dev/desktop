import { useState } from "react";
import { Download } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { settingsApi } from "@/lib/api.ts";
import {
  CHATGPT_DESKTOP_URLS,
  detectArm,
  detectDesktopOs,
  thisMachineDownloadUrl,
  thisMachineLabel,
} from "@/config/chatgptDesktop.ts";

export function ChatgptDesktopDownload() {
  const os = detectDesktopOs();
  const arm = detectArm();
  const [busy, setBusy] = useState(false);
  const links: { id: string; label: string; url: string }[] = [
    { id: "macos", label: "macOS", url: CHATGPT_DESKTOP_URLS.macos },
    { id: "windows", label: "Windows", url: CHATGPT_DESKTOP_URLS.windows },
    {
      id: "deb",
      label: "Linux .deb",
      url: arm
        ? CHATGPT_DESKTOP_URLS.linuxDebArm64
        : CHATGPT_DESKTOP_URLS.linuxDebAmd64,
    },
    {
      id: "rpm",
      label: "Linux .rpm",
      url: arm
        ? CHATGPT_DESKTOP_URLS.linuxRpmArm64
        : CHATGPT_DESKTOP_URLS.linuxRpmX64,
    },
  ];

  async function open(url: string) {
    setBusy(true);
    try {
      await settingsApi.openExternal(url);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mt-6 flex flex-col gap-3 rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <p className="text-sm font-medium">ChatGPT desktop (Codex included)</p>
        <p className="mt-0.5 text-xs text-muted-foreground">
          Official OpenAI app for macOS, Windows, and Linux. Codex.app updates
          into this same download.
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {links.map((link) => (
          <Button
            key={link.id}
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => void open(link.url)}
          >
            {link.label}
          </Button>
        ))}
        <Button
          size="sm"
          disabled={busy}
          onClick={() => void open(thisMachineDownloadUrl(os))}
        >
          <Download className="h-3.5 w-3.5" />
          {thisMachineLabel(os)}
        </Button>
      </div>
    </div>
  );
}
