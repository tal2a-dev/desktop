import type { ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "@/i18n";

interface FullScreenPanelProps {
  isOpen: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  motionPreset?: "fade" | "slide-from-right";
  contentClassName?: string;
}

export function FullScreenPanel({
  isOpen,
  title,
  onClose,
  children,
  footer,
  contentClassName,
}: FullScreenPanelProps) {
  const { t } = useTranslation();
  if (!isOpen) return null;
  return (
    <div className="fixed inset-0 z-[80] flex flex-col bg-background">
      <div className="flex items-center gap-3 border-b border-border-default px-4 py-3">
        <Button variant="ghost" size="icon" onClick={onClose} aria-label={t("common.back")}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h2 className="text-base font-semibold">{title}</h2>
      </div>
      <div className={contentClassName ?? "flex-1 overflow-y-auto px-6 py-6 space-y-6"}>
        {children}
      </div>
      {footer}
    </div>
  );
}
