import { useCallback, useEffect, useState } from "react";
import { ArrowUpCircle, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "./ui/button.tsx";
import {
  checkForUpdate,
  getCurrentVersion,
  installUpdateAndRelaunch,
  type UpdateStatus,
} from "../lib/updater.ts";

export function AppUpdater({ autoCheck = false }: { autoCheck?: boolean }) {
  const [current, setCurrent] = useState<string>("");
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [busy, setBusy] = useState<"check" | "install" | null>(null);

  const runCheck = useCallback(async (quiet: boolean) => {
    setBusy("check");
    try {
      const next = await checkForUpdate();
      setStatus(next);
      if (next.status === "available") {
        toast.message(`tal2a ${next.availableVersion} is available`, {
          description: `This install is ${next.currentVersion}.`,
        });
      } else if (!quiet && next.status === "up-to-date") {
        toast.success(`You're on ${next.currentVersion}`);
      }
    } catch (e) {
      if (!quiet) toast.error(String(e));
    } finally {
      setBusy(null);
    }
  }, []);

  useEffect(() => {
    void getCurrentVersion().then(setCurrent);
  }, []);

  useEffect(() => {
    if (!autoCheck) return;
    const t = window.setTimeout(() => void runCheck(true), 1500);
    return () => window.clearTimeout(t);
  }, [autoCheck, runCheck]);

  const install = async () => {
    if (status?.status !== "available") return;
    setBusy("install");
    try {
      await installUpdateAndRelaunch(status.update);
    } catch (e) {
      toast.error(String(e));
      setBusy(null);
    }
  };

  const available = status?.status === "available";

  return (
    <div className="rounded-xl glass-card p-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-base font-semibold">Desktop updates</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            Installed version {current || "…"}. New builds publish from GitHub
            Releases on every push to main.
          </p>
          {available && (
            <p className="mt-2 text-sm">
              {status.availableVersion} is ready
              {status.notes ? ` — ${status.notes}` : "."}
            </p>
          )}
        </div>
        <div className="flex shrink-0 gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={busy !== null}
            onClick={() => void runCheck(false)}
          >
            {busy === "check" ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              "Check"
            )}
          </Button>
          {available && (
            <Button
              size="sm"
              disabled={busy !== null}
              onClick={() => void install()}
            >
              {busy === "install" ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <ArrowUpCircle className="h-4 w-4" />
              )}
              <span className="ml-1">Install & restart</span>
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

export function AutoUpdateCheck() {
  return <SilentCheck />;
}

function SilentCheck() {
  useEffect(() => {
    const t = window.setTimeout(() => {
      void checkForUpdate()
        .then((status) => {
          if (status.status !== "available") return;
          toast.message(`tal2a ${status.availableVersion} is available`, {
            description: "Settings → About → Install & restart",
            duration: 8000,
          });
        })
        .catch(() => {
          /* updater endpoint missing until the first GitHub release */
        });
    }, 2000);
    return () => window.clearTimeout(t);
  }, []);
  return null;
}
