'use client';

import dynamic from 'next/dynamic';

const ClientOnlyApp = dynamic(() => import('@/components/ClientOnlyApp'), { 
  ssr: false,
  loading: () => <div>Loading application...</div>
});

export default function Home() {
  return <ClientOnlyApp />;
}