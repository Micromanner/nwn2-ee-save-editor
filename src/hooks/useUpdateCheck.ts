import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { fetch } from '@tauri-apps/plugin-http';

const RELEASES_API = 'https://api.github.com/repos/Micromanner/nwn2-ee-save-editor/releases/latest';
const DISMISSED_KEY = 'nwn2ee.dismissedUpdateVersion';

export interface UpdateInfo {
  current: string;
  latest: string;
  htmlUrl: string;
}

function parseVersion(v: string): number[] {
  return v.replace(/^v/, '').split('.').map(n => {
    const parsed = parseInt(n, 10);
    return Number.isFinite(parsed) ? parsed : 0;
  });
}

function isNewer(latest: string, current: string): boolean {
  const a = parseVersion(latest);
  const b = parseVersion(current);
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff > 0;
  }
  return false;
}

export function useUpdateCheck() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const [current, res] = await Promise.all([
          getVersion(),
          fetch(RELEASES_API, {
            method: 'GET',
            headers: { Accept: 'application/vnd.github+json' },
          }),
        ]);
        if (!res.ok) return;
        const data = await res.json() as { tag_name?: string; html_url?: string };
        const latest = data.tag_name?.replace(/^v/, '');
        if (!latest || !data.html_url) return;
        if (!isNewer(latest, current)) return;
        if (sessionStorage.getItem(DISMISSED_KEY) === latest) return;
        if (!cancelled) setInfo({ current, latest, htmlUrl: data.html_url });
      } catch {
        // offline, rate-limited, or blocked — silently skip
      }
    })();

    return () => { cancelled = true; };
  }, []);

  const dismiss = () => {
    if (info) sessionStorage.setItem(DISMISSED_KEY, info.latest);
    setInfo(null);
  };

  return { info, dismiss };
}
