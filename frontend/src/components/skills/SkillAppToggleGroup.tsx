import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { APP_IDS, APP_ICON_MAP } from "@/config/appConfig";
import type { AppId } from "@/lib/api/types";

interface SkillAppToggleGroupProps {
  apps: Partial<Record<AppId, boolean>>;
  onToggle: (app: AppId, enabled: boolean) => void;
  appIds?: AppId[];
  disabled?: boolean;
}

export function SkillAppToggleGroup({
  apps,
  onToggle,
  appIds = APP_IDS,
  disabled = false,
}: SkillAppToggleGroupProps) {
  return (
    <div className="flex flex-shrink-0 items-center gap-1.5">
      {appIds.map((app) => {
        const { label, icon, activeClass } = APP_ICON_MAP[app];
        const enabled = apps[app];
        return (
          <Tooltip key={app}>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => onToggle(app, !enabled)}
                disabled={disabled}
                aria-label={label}
                aria-pressed={Boolean(enabled)}
                className={`flex h-7 w-7 items-center justify-center rounded-lg transition-all ${
                  enabled ? activeClass : "opacity-35 hover:opacity-70"
                } disabled:cursor-not-allowed`}
              >
                {icon}
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>
                {label}
                {enabled ? " ✓" : ""}
              </p>
            </TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
}
