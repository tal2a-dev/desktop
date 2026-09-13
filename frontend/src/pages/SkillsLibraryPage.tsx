import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SkillInfo {
  name: string;
  agent: string;
}

const MAX_ROWS = 80;

export function SkillsLibraryPage() {
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<SkillInfo[]>("scan_skills")
      .then(setSkills)
      .catch(() => setSkills([]))
      .finally(() => setLoading(false));
  }, []);

  if (loading)
    return <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>;

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
            {skills.length} skills installed across {Object.keys(totals).length}{" "}
            agents
          </p>
        </div>
        <input
          className="search-input"
          placeholder="Filter skills…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      {skills.length === 0 && (
        <p className="page-sub">No installed skills found.</p>
      )}

      {needle === "" && (
        <div className="lib-chips">
          {Object.entries(totals).map(([agent, n]) => (
            <span className="lib-chip" key={agent}>
              {agent} <b>{n}</b>
            </span>
          ))}
        </div>
      )}

      {Object.entries(byAgent).map(([agent, names]) => (
        <div key={agent} className="lib-group">
          <h3 className="section-heading">
            {agent} <span className="lib-count">{names.length}</span>
          </h3>
          <div className="lib-list">
            {names.slice(0, MAX_ROWS).map((n) => (
              <div className="lib-row compact" key={`${agent}-${n}`}>
                <span className="lib-name">{n}</span>
              </div>
            ))}
            {names.length > MAX_ROWS && (
              <div className="lib-row compact">
                <span className="lib-more">
                  +{names.length - MAX_ROWS} more — refine the filter to see
                  them
                </span>
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
