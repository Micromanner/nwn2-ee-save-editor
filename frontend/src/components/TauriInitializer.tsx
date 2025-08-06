'use client';

import { useEffect } from 'react';
import { useTauri } from '@/providers/TauriProvider';

// This component just triggers Tauri initialization on app load
export default function TauriInitializer() {
  const { isLoading, isAvailable } = useTauri();
  
  useEffect(() => {
    console.log('🚀 TauriInitializer: App loaded, Tauri status:', { isLoading, isAvailable });
  }, [isLoading, isAvailable]);

  return null; // This component renders nothing
}