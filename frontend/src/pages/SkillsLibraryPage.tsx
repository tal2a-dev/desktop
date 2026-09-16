import { useRef, useState } from "react";
import {
  Download,
  FolderArchive,
  History,
  Loader2,
  RefreshCw,
  Search,
  Settings,
} from "lucide-react";
import UnifiedSkillsPanel, {
  type SkillsCheckUpdatesState,
  type UnifiedSkillsPanelHandle,
} from "@/components/skills/UnifiedSkillsPanel";
import {
  SkillsPage,
  getSkillsPageHeaderActions,
  type SkillsPageHandle,
  type SkillsPageSource,
} from "@/components/skills/SkillsPage";
import { Button } from "@/components/ui/button";
import { useScanUnmanagedSkills } from "@/hooks/useSkills";
import { useTranslation } from "@/i18n";

export function SkillsLibraryPage() {
  const { t } = useTranslation();
  const panelRef = useRef<UnifiedSkillsPanelHandle>(null);
  const pageRef = useRef<SkillsPageHandle>(null);
  const [view, setView] = useState<"manage" | "discover">("manage");
  const [source, setSource] = useState<SkillsPageSource>("repos");
  const [busy, setBusy] = useState(false);
  const [checkState, setCheckState] = useState<SkillsCheckUpdatesState>({
    isChecking: false,
    hasSkills: false,
  });
  const { data: unmanagedSkills } = useScanUnmanagedSkills();
  const hasUnmanagedSkills = (unmanagedSkills?.length ?? 0) > 0;

  return (
    <div className="flex h-full min-h-0 flex-col px-6 py-4">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div>
          <h2>{view === "manage" ? t("skills.manage") : t("skills.discover")}</h2>
          <p className="page-sub">{t("skills.description")}</p>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          {view === "discover" ? (
            <>
              <Button variant="ghost" size="sm" onClick={() => setView("manage")}>
                {t("common.back")}
              </Button>
              {getSkillsPageHeaderActions(source).map(({ key, labelKey, Icon, execute }) => (
                <Button
                  key={key}
                  variant="ghost"
                  size="sm"
                  onClick={() => execute(pageRef.current)}
                >
                  {key === "manage-repos" ? (
                    <Settings className="mr-2 h-4 w-4" />
                  ) : (
                    <Icon className="mr-2 h-4 w-4" />
                  )}
                  {t(labelKey)}
                </Button>
              ))}
            </>
          ) : (
            <>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy || checkState.isChecking || !checkState.hasSkills}
                onClick={() => panelRef.current?.checkUpdates()}
              >
                {checkState.isChecking ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <RefreshCw className="mr-2 h-4 w-4" />
                )}
                {checkState.isChecking
                  ? t("skills.checkingUpdates")
                  : t("skills.checkUpdates")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={() => panelRef.current?.openRestoreFromBackup()}
              >
                <History className="mr-2 h-4 w-4" />
                {t("skills.restoreFromBackup.button")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={() => panelRef.current?.openInstallFromZip()}
              >
                <FolderArchive className="mr-2 h-4 w-4" />
                {t("skills.installFromZip.button")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy}
                className="relative"
                title={hasUnmanagedSkills ? t("skills.unmanagedAvailable") : undefined}
                onClick={() => panelRef.current?.openImport()}
              >
                <Download className="mr-2 h-4 w-4" />
                {t("skills.import")}
                {hasUnmanagedSkills && (
                  <span
                    className="absolute right-1 top-1 h-2 w-2 rounded-full bg-green-500"
                    aria-hidden="true"
                  />
                )}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={() => panelRef.current?.openDiscovery()}
              >
                <Search className="mr-2 h-4 w-4" />
                {t("skills.discover")}
              </Button>
            </>
          )}
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col">
        {view === "manage" ? (
          <UnifiedSkillsPanel
            ref={panelRef}
            currentApp="claude"
            onOpenDiscovery={() => {
              setSource("repos");
              setView("discover");
            }}
            onInteractionBlockedChange={setBusy}
            onCheckUpdatesStateChange={setCheckState}
          />
        ) : (
          <SkillsPage
            ref={pageRef}
            initialApp="claude"
            onSourceChange={setSource}
          />
        )}
      </div>
    </div>
  );
}
