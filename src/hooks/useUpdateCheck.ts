import { useCallback, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { fetch } from '@tauri-apps/plugin-http';

const RELEASES_API = 'https://api.github.com/repos/Micromanner/nwn2-ee-save-editor/releases/latest';

export type UpdateCheckResult =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'upToDate'; current: string }
  | { kind: 'available'; current: string; latest: string; htmlUrl: string }
  | { kind: 'error' };

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
  const [result, setResult] = useState<UpdateCheckResult>({ kind: 'idle' });

  const checkNow = useCallback(async () => {
    setResult({ kind: 'checking' });
    try {
      const [current, res] = await Promise.all([
        getVersion(),
        fetch(RELEASES_API, {
          method: 'GET',
          headers: { Accept: 'application/vnd.github+json' },
        }),
      ]);
      if (!res.ok) {
        setResult({ kind: 'error' });
        return;
      }
      const data = await res.json() as { tag_name?: string; html_url?: string };
      const latest = data.tag_name?.replace(/^v/, '');
      if (!latest || !data.html_url) {
        setResult({ kind: 'error' });
        return;
      }
      if (isNewer(latest, current)) {
        setResult({ kind: 'available', current, latest, htmlUrl: data.html_url });
      } else {
        setResult({ kind: 'upToDate', current });
      }
    } catch {
      setResult({ kind: 'error' });
    }
  }, []);

  return { result, checkNow };
}
