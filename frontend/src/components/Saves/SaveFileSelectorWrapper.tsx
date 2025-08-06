'use client';

import dynamic from 'next/dynamic';

// Import SaveFileSelector with SSR disabled since it needs Tauri APIs
const SaveFileSelector = dynamic(() => import('./SaveFileSelector').then(mod => ({ default: mod.SaveFileSelector })), {
  ssr: false,
  loading: () => <div className="text-sm text-gray-500">Loading file selector...</div>
});

export default function SaveFileSelectorWrapper() {
  return <SaveFileSelector />;
}