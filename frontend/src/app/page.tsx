'use client';

import dynamic from 'next/dynamic';

// Disable SSR completely for desktop app
const ClientOnlyApp = dynamic(() => {
  console.log('📦 page.tsx: Attempting to import ClientOnlyApp...');
  return import('@/components/ClientOnlyApp');
}, { 
  ssr: false,
  loading: () => {
    console.log('📦 page.tsx: Dynamic loading fallback shown');
    return <div>Loading application...</div>;
  }
});

export default function Home() {
  console.log('📦 page.tsx: Home component rendering');
  return <ClientOnlyApp />;
}