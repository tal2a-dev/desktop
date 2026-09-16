import lockup from "../brand/tal2a-lockup.png";
import { cn } from "../lib/utils.ts";

export function BrandLockup({
  className,
  heightClass = "h-8",
}: {
  className?: string;
  heightClass?: string;
}) {
  return (
    <img
      src={lockup}
      alt="tal2a"
      className={cn(heightClass, "w-auto select-none", className)}
    />
  );
}
