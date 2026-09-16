import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button.tsx";
import { Label } from "@/components/ui/label.tsx";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select.tsx";
import { Switch } from "@/components/ui/switch.tsx";
import { LOG_LEVELS, type LogLevel } from "./types.ts";

const LEVEL_LABEL: Record<LogLevel, string> = {
  error: "Error",
  warn: "Warning",
  info: "Info",
  debug: "Debug",
  trace: "Trace",
};

const LEVEL_DESC: Record<LogLevel, { color: string; text: string }> = {
  error: { color: "text-red-500", text: "Critical errors only" },
  warn: { color: "text-orange-500", text: "Errors + warnings" },
  info: { color: "text-blue-500", text: "General operation info (default)" },
  debug: {
    color: "text-green-500",
    text: "Detailed info including request/response",
  },
  trace: { color: "text-gray-500", text: "All logs, most verbose" },
};

interface LogConfigPanelProps {
  enabled: boolean;
  level: LogLevel;
  onEnabledChange: (enabled: boolean) => void;
  onLevelChange: (level: LogLevel) => void;
  onOpenLogs: () => void;
}

export function LogConfigPanel({
  enabled,
  level,
  onEnabledChange,
  onLevelChange,
  onOpenLogs,
}: LogConfigPanelProps) {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-4">
        <div className="space-y-0.5">
          <Label>Enable diagnostic logs</Label>
          <p className="text-xs text-muted-foreground">
            Write NAPI Desktop diagnostic logs to the logs directory
          </p>
        </div>
        <Switch checked={enabled} onCheckedChange={onEnabledChange} />
      </div>

      <div className="flex items-center justify-between gap-4">
        <div className="space-y-0.5">
          <Label>Log level</Label>
          <p className="text-xs text-muted-foreground">
            Set the minimum log level to output
          </p>
        </div>
        <Select
          value={level}
          disabled={!enabled}
          onValueChange={(value) => onLevelChange(value as LogLevel)}
        >
          <SelectTrigger className="w-[120px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {LOG_LEVELS.map((item) => (
              <SelectItem key={item} value={item}>
                {LEVEL_LABEL[item]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-1.5 rounded-lg bg-muted/50 p-4 text-xs">
        <p className="mb-2 font-medium text-muted-foreground">
          Log level descriptions:
        </p>
        <div className="grid gap-1 text-muted-foreground">
          {LOG_LEVELS.map((item) => (
            <p key={item}>
              <span className={`font-mono ${LEVEL_DESC[item].color}`}>
                {item}
              </span>{" "}
              - {LEVEL_DESC[item].text}
            </p>
          ))}
        </div>
      </div>

      <div className="flex justify-end">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 gap-1.5 text-xs"
          onClick={onOpenLogs}
        >
          <FolderOpen className="h-3.5 w-3.5" />
          Open logs directory
        </Button>
      </div>
    </div>
  );
}
