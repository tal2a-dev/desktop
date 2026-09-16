/** Official ChatGPT desktop (Chat + Work + Codex) installers.
 *  Codex.app users update in-place to this app. Sources:
 *  https://chatgpt.com/download/
 *  https://learn.chatgpt.com/docs/linux/linux-app
 *  https://learn.chatgpt.com/docs/enterprise/windows-deployment
 *  https://persistent.oaistatic.com/codex-app-prod/appcast.xml
 */
export const CHATGPT_DESKTOP_LANDING = "https://chatgpt.com/download/";

export const CHATGPT_DESKTOP_URLS = {
  landing: CHATGPT_DESKTOP_LANDING,
  macos: CHATGPT_DESKTOP_LANDING,
  windows: "https://get.microsoft.com/installer/download/9PLM9XGG6VKS?cid=website_cta_psi",
  windowsX64:
    "https://persistent.oaistatic.com/codex-app-prod/ChatGPT-x64.msix",
  windowsArm64:
    "https://persistent.oaistatic.com/codex-app-prod/ChatGPT-arm64.msix",
  linuxDebAmd64:
    "https://persistent.oaistatic.com/codex-app-prod/linux/deb/latest/chatgpt_amd64.deb",
  linuxDebArm64:
    "https://persistent.oaistatic.com/codex-app-prod/linux/deb/latest/chatgpt_arm64.deb",
  linuxRpmX64:
    "https://persistent.oaistatic.com/codex-app-prod/linux/rpm/latest/chatgpt.x86_64.rpm",
  linuxRpmArm64:
    "https://persistent.oaistatic.com/codex-app-prod/linux/rpm/latest/chatgpt.aarch64.rpm",
} as const;

export type DesktopOs = "macos" | "windows" | "linux";

export function detectDesktopOs(ua = navigator.userAgent): DesktopOs {
  if (/Mac OS X|Macintosh/i.test(ua)) return "macos";
  if (/Windows NT/i.test(ua)) return "windows";
  return "linux";
}

export function detectArm(ua = navigator.userAgent): boolean {
  return /aarch64|arm64|Apple Silicon/i.test(ua) || (navigator as Navigator & { userAgentData?: { architecture?: string } }).userAgentData?.architecture === "arm";
}

export function thisMachineDownloadUrl(os = detectDesktopOs(), arm = detectArm()): string {
  if (os === "macos") return CHATGPT_DESKTOP_URLS.macos;
  if (os === "windows") return CHATGPT_DESKTOP_URLS.windows;
  return arm
    ? CHATGPT_DESKTOP_URLS.linuxDebArm64
    : CHATGPT_DESKTOP_URLS.linuxDebAmd64;
}

export function thisMachineLabel(os = detectDesktopOs()): string {
  if (os === "macos") return "Download for macOS";
  if (os === "windows") return "Download for Windows";
  return "Download Linux .deb";
}
