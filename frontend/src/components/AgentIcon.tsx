import claudeSvg from "@/icons/claude.svg?url";
import openaiSvg from "@/icons/openai.svg?url";
import geminiSvg from "@/icons/gemini.svg?url";
import clawSvg from "@/icons/claw.svg?url";
import grokSvg from "@/icons/grok.svg?url";
import opencodeSvg from "@/icons/opencode-logo-light.svg?url";
import hermesPng from "@/icons/hermes.png";
import qwenSvg from "@/icons/qwen.svg?url";
import githubSvg from "@/icons/github.svg?url";
import copilotSvg from "@/icons/copilot.svg?url";
import xaiSvg from "@/icons/xai.svg?url";
import googleSvg from "@/icons/google.svg?url";
import { cn } from "@/lib/utils.ts";

const ICONS: Record<string, { src: string; invert?: boolean; alt: string }> = {
  claude: { src: claudeSvg, alt: "Claude Code" },
  codex: { src: openaiSvg, invert: true, alt: "Codex" },
  gemini: { src: geminiSvg, alt: "Gemini" },
  grok: { src: grokSvg, invert: true, alt: "Grok" },
  grokbuild: { src: grokSvg, invert: true, alt: "Grok Build" },
  "claude-desktop": { src: claudeSvg, alt: "Claude Desktop" },
  mcode: { src: copilotSvg, alt: "MiniMax Code" },
  opencode: { src: opencodeSvg, invert: true, alt: "OpenCode" },
  openclaw: { src: clawSvg, alt: "OpenClaw" },
  hermes: { src: hermesPng, alt: "Hermes" },
  qwen: { src: qwenSvg, alt: "Qwen" },
  pi: { src: openaiSvg, invert: true, alt: "Pi" },
  cursor: { src: copilotSvg, alt: "Cursor" },
  cline: { src: githubSvg, invert: true, alt: "Cline" },
  continue: { src: githubSvg, invert: true, alt: "Continue" },
  goose: { src: openaiSvg, invert: true, alt: "Goose" },
  factory: { src: xaiSvg, invert: true, alt: "Factory" },
  roo: { src: copilotSvg, alt: "Roo Code" },
  kilocode: { src: copilotSvg, alt: "Kilo Code" },
  windsurf: { src: googleSvg, alt: "Windsurf" },
};

export function AgentIcon({
  id,
  name,
  size = 20,
  className,
}: {
  id: string;
  name: string;
  size?: number;
  className?: string;
}) {
  const icon = ICONS[id];
  if (!icon) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center rounded-lg bg-muted text-xs font-semibold",
          className,
        )}
        style={{ width: size, height: size }}
      >
        {name.charAt(0)}
      </span>
    );
  }
  return (
    <img
      src={icon.src}
      width={size}
      height={size}
      alt={icon.alt}
      className={cn(
        "flex-shrink-0 object-contain",
        icon.invert && "dark:brightness-0 dark:invert",
        className,
      )}
    />
  );
}
