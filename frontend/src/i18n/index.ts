type Dict = { [key: string]: string | Dict };

const EN: Dict = {
  common: {
    add: "Add",
    delete: "Delete",
    cancel: "Cancel",
    confirm: "Confirm",
    close: "Close",
    loading: "Loading...",
    success: "Success",
    error: "Error",
    clear: "Clear",
    view: "View",
    back: "Back",
    refresh: "Refresh",
    import: "Import",
    enableAllForApp: "Enable all for {{app}}",
    disableAllForApp: "Disable all for {{app}}",
    bulkToggleFailed: "Failed to update {{count}} item(s)",
  },
  skills: {
    manage: "Skills",
    title: "Skills Management",
    description:
      "Discover and install skills from GitHub repositories and skills.sh",
    refresh: "Refresh",
    refreshing: "Refreshing...",
    repoManager: "Repository Management",
    count: "{{count}} skills",
    empty: "No skills available",
    emptyDescription: "Add skill repositories to discover available skills",
    addRepo: "Add Skill Repository",
    loading: "Loading...",
    installed: "Installed",
    install: "Install",
    installing: "Installing...",
    uninstall: "Uninstall",
    uninstalling: "Uninstalling...",
    view: "View",
    installSuccess: "Skill {{name}} installed",
    installFailed: "Failed to install",
    uninstallSuccess: "Skill {{name}} uninstalled",
    uninstallPiPreserved:
      "Pi's copy could not be verified as managed and was left in place: {{path}}",
    uninstallPiCleanupIncomplete:
      "Pi cleanup could not be completed. Check the Pi Skills directory manually.",
    uninstallFailed: "Failed to uninstall",
    update: "Update",
    updating: "Updating...",
    updateAvailable: "Update",
    updateSuccess: "Skill {{name}} updated to latest version",
    updateFailed: "Failed to update",
    checkUpdates: "Check Updates",
    checkingUpdates: "Checking...",
    noUpdates: "All skills are up to date",
    updatesFound: "{{count}} skill(s) have updates available",
    updateAll: "Update All ({{count}})",
    updatingAll: "Updating...",
    updateAllSuccess: "Successfully updated {{count}} skill(s)",
    error: {
      skillNotFound: "Skill not found: {{directory}}",
      missingRepoInfo: "Missing repository info (owner or name)",
      downloadTimeout:
        "Download repository {{owner}}/{{name}} timeout ({{timeout}}s)",
      skillDirNotFound: "Skill directory not found: {{path}}",
      directoryConflict:
        "Skill directory '{{directory}}' is already occupied by {{existing_repo}}, cannot install from {{new_repo}}",
      emptyArchive: "Downloaded archive is empty",
      invalidRepoRef: "Invalid repository reference: {{owner}}/{{name}}",
      archiveTooLarge:
        "Archive exceeds the {{limit_mb}} MB extraction limit; aborted",
      archiveTooManyEntries:
        "Archive has too many entries ({{count}}); the limit is {{limit}}",
      downloadFailed: "Download failed: HTTP {{status}}",
      http403: "GitHub access restricted, possibly rate limited",
      http404: "Repository or branch not found, please check URL",
      http429: "Too many requests, please wait and retry",
      getHomeDirFailed: "Unable to get user home directory",
      noSkillsInZip:
        "No skills found in ZIP file (requires SKILL.md file)",
      unknownError: "Unknown error",
      suggestion: {
        checkNetwork: "Please check network connection",
        checkProxy: "Consider configuring HTTP proxy",
        retryLater: "Please retry later",
        checkRepoUrl: "Please check repository URL and branch name",
        checkPermission: "Please check directory permissions",
        uninstallFirst:
          "Please uninstall the existing skill with the same name first",
        checkZipContent:
          "Please verify the ZIP file contains valid skill directories (with SKILL.md files)",
      },
    },
    repo: {
      title: "Manage Skill Repositories",
      description: "Add or remove GitHub skill repository sources",
      url: "Repository URL",
      urlPlaceholder: "owner/name or https://github.com/owner/name",
      branch: "Branch",
      branchPlaceholder: "main",
      add: "Add Repository",
      list: "Added Repositories",
      empty: "No repositories",
      invalidUrl: "Invalid repository URL format",
      addSuccess:
        "Repository {{owner}}/{{name}} added, detected {{count}} skills",
      addFailed: "Failed to add",
      removeSuccess: "Repository {{owner}}/{{name}} removed",
      removeFailed: "Failed to remove",
      skillCount: "{{count}} skills detected",
    },
    search: "Search Skills",
    searchPlaceholder: "Search skill name or repo...",
    installedSearchPlaceholder:
      "Search installed skill name, description, or repo...",
    installedSearchAriaLabel: "Search installed skills",
    noInstalledSearchResults: "No installed skills match your search",
    searchSource: {
      repos: "Repos",
      skillssh: "skills.sh",
    },
    skillssh: {
      searchPlaceholder: "Search skills.sh (min 2 chars)...",
      installs: "{{count}} installs",
      loadMore: "Load More",
      loading: "Searching skills.sh...",
      noResults: 'No skills found for "{{query}}"',
      error: "Failed to search skills.sh",
      poweredBy: "Powered by skills.sh",
    },
    filter: {
      placeholder: "Filter by status",
      all: "All",
      installed: "Installed",
      uninstalled: "Not installed",
      repo: "Filter by repo",
      allRepos: "All repos",
    },
    noResults: "No matching skills found",
    noInstalled: "No skills installed",
    noInstalledDescription:
      "Discover and install skills from repositories, or import existing skills",
    discover: "Discover Skills",
    import: "Import Existing",
    importDescription:
      "Select skills to import into NAPI Desktop unified management",
    importSuccess: "Successfully imported {{count}} skills",
    importSelected: "Import Selected ({{count}})",
    noUnmanagedFound:
      "No skills to import found. All skills are already managed.",
    unmanagedAvailable: "Skills available to import",
    local: "Local",
    uninstallConfirm:
      'Are you sure you want to uninstall "{{name}}"? This will remove the skill from all apps and create a local backup first.',
    uninstallInMainPanel: "Please uninstall skills from the main panel",
    notFound: "Skill not found",
    backup: {
      location: "Backup location: {{path}}",
    },
    restoreFromBackup: {
      button: "Restore Backup",
      title: "Restore From Backup",
      description:
        "Choose a Skills backup to restore its files locally and add it back to the current list.",
      empty: "No Skills backups available to restore",
      createdAt: "Backed up at",
      path: "Backup path",
      restore: "Restore",
      restoring: "Restoring...",
      delete: "Delete",
      deleting: "Deleting...",
      deleteSuccess: "Deleted backup for {{name}}",
      deleteFailed: "Failed to delete skill backup",
      deleteConfirmTitle: "Delete Backup",
      deleteConfirmMessage:
        'Are you sure you want to delete the backup for "{{name}}"? This action cannot be undone.',
      success: "Skill {{name}} restored from backup",
      failed: "Failed to restore from backup",
    },
    installFromZip: {
      button: "Install from ZIP",
      installing: "Installing...",
      successSingle: "Skill {{name}} installed",
      successMultiple: "Successfully installed {{count}} skills",
      noSkillsFound:
        "No skills found in ZIP file (requires SKILL.md file)",
    },
  },
};

function lookup(obj: Dict, path: string): string | undefined {
  const parts = path.split(".");
  let cur: string | Dict | undefined = obj;
  for (const part of parts) {
    if (typeof cur !== "object" || cur == null) return undefined;
    cur = cur[part];
  }
  return typeof cur === "string" ? cur : undefined;
}

export function useTranslation() {
  const t = (key: string, vars?: Record<string, string | number>) => {
    let value = lookup(EN, key) ?? key;
    if (vars) {
      for (const [name, raw] of Object.entries(vars)) {
        value = value.replaceAll(`{{${name}}}`, String(raw));
      }
    }
    return value;
  };
  return { t };
}
