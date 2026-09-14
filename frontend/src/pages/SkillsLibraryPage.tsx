import { useState, useEffect, useCallback } from "react";
import { invoke } from "../lib/tauri.ts";

interface SkillInfo {
  name: string;
  agent: string;
}

const PAGE_STEP = 80;

// ponytail: mirrors the scan_skills source dirs in src-tauri/src/lib.rs so rows can
// show/copy each SKILL.md path without a new Tauri command. Update both if sources change.
const SKILL_DIRS: Record<string, string> = {
  "Claude Code": ".claude/skills",
  OpenCode: ".config/opencode/skills",
  Cline: ".cline/skills",
  Roo: ".roo/skills",
  "Kilo Code": ".kilocode/skills",
  "Grok CLI": ".grok/skills",
  "Qwen Code": ".qwen/skills",
  Hermes: ".hermes/skills",
};

function skillRelPath(agent: string, name: string): string {
  return `~/${SKILL_DIRS[agent] ?? ".skills"}/${name}/SKILL.md`;
}

export function SkillsLibraryPage() {
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(true);
  const [limits, setLimits] = useState<Record<string, number>>({});
  const [copied, setCopied] = useState<string | null>(null);

  const load = useCallback(() => {
    setLoading(true);
    invoke<SkillInfo[]>("scan_skills")
      .then(setSkills)
      .catch(() => setSkills([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const copyText = async (key: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(key);
      window.setTimeout(
        () => setCopied((cur) => (cur === key ? null : cur)),
        1500,
      );
    } catch {
      // Clipboard unavailable — the name/path text stays selectable.
    }
  };

  if (loading)
    return (
      <div>
        <div className="page-head">
          <div>
            <h2>Skills Library</h2>
            <p className="page-sub">Scanning installed skills…</p>
          </div>
        </div>
        <div className="skeleton-list" role="status" aria-label="Loading skills">
          {Array.from({ length: 8 }).map((_, i) => (
            <div className="skeleton" key={i} style={{ height: 33 }} />
          ))}
        </div>
      </div>
    );

  const needle = q.trim().toLowerCase();
  const matches = needle
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(needle) ||
          s.agent.toLowerCase().includes(needle),
      )
    : skills;

  const byAgent = matches.reduce<Record<string, string[]>>((acc, s) => {
    (acc[s.agent] ||= []).push(s.name);
    return acc;
  }, {});

  const totals = skills.reduce<Record<string, number>>((acc, s) => {
    acc[s.agent] = (acc[s.agent] || 0) + 1;
    return acc;
  }, {});

  return (
    <div>
      <div className="page-head">
        <div>
          <h2>Skills Library</h2>
          <p className="page-sub">
            {skills.length} skills across {Object.keys(totals).length} agents (
            {matches.length} shown)
          </p>
        </div>
        <input
          className="search-input"
          type="search"
          aria-label="Filter skills by name or agent"
          placeholder="Filter skills…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      {skills.length === 0 && (
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ✦
          </div>
          <h3>No installed skills found</h3>
          <p>
            A skill is a directory containing SKILL.md in one of these
            locations:
          </p>
          <div
            style={{
              display: "inline-block",
              textAlign: "left",
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-muted)",
              marginBottom: 16,
            }}
          >
            {Object.values(SKILL_DIRS).map((d) => (
              <div key={d}>
                ~/{d}/&lt;name&gt;/SKILL.md
              </div>
            ))}
          </div>
          <br />
          <button className="btn-secondary" onClick={load}>
            Rescan
          </button>
        </div>
      )}

      {skills.length > 0 && matches.length === 0 && (
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ∅
          </div>
          <h3>No skills match “{q.trim()}”</h3>
          <p>Try a different skill name or agent.</p>
          <button className="btn-secondary" onClick={() => setQ("")}>
            Clear filter
          </button>
        </div>
      )}

      {needle === "" && skills.length > 0 && (
        <div className="lib-chips">
          {Object.entries(totals).map(([agent, n]) => (
            <span className="chip" key={agent}>
              {agent} <b>{n}</b>
            </span>
          ))}
        </div>
      )}

      {Object.entries(byAgent).map(([agent, names]) => {
        const limit = limits[agent] ?? PAGE_STEP;
        const visible = names.slice(0, limit);
        const remaining = names.length - limit;
        const total = totals[agent] ?? names.length;
        return (
          <div key={agent} className="lib-group">
            <h3 className="section-heading">
              {agent}{" "}
              <span
                className="chip"
                title={`${visible.length} shown of ${total}`}
              >
                {visible.length}/{total}
              </span>
            </h3>
            <div className="lib-list">
              {visible.map((n) => {
                const rel = skillRelPath(agent, n);
                const nameKey = `${agent}-${n}-name`;
                const pathKey = `${agent}-${n}-path`;
                return (
                  <div className="lib-row compact" key={`${agent}-${n}`}>
                    <span className="lib-name" title={rel}>
                      {n}
                    </span>
                    <span
                      className="lib-target"
                      title={rel}
                      style={{
                        flex: 1,
                        minWidth: 0,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {rel}
                    </span>
                    <span
                      style={{
                        marginLeft: "auto",
                        display: "inline-flex",
                        gap: 6,
                        flexShrink: 0,
                      }}
                    >
                      <button
                        className="btn-inline"
                        title="Copy skill name"
                        onClick={() => copyText(nameKey, n)}
                      >
                        {copied === nameKey ? "Copied" : "Copy name"}
                      </button>
                      <button
                        className="btn-inline"
                        title="Copy SKILL.md path"
                        onClick={() => copyText(pathKey, rel)}
                      >
                        {copied === pathKey ? "Copied" : "Copy path"}
                      </button>
                    </span>
                  </div>
                );
              })}
            </div>
            {remaining > 0 && (
              <button
                className="btn-inline load-more"
                onClick={() =>
                  setLimits((m) => ({ ...m, [agent]: limit + PAGE_STEP }))
                }
              >
                Show more ({remaining} remaining)
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
