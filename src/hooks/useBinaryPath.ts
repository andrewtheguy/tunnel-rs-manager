import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function useBinaryPath() {
  const [binaryPath, setBinaryPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const path = await invoke<string | null>('get_binary_path');
      setBinaryPath(path);
    } catch (e) {
      console.error('Failed to get binary path:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const selectBinaryPath = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: 'Select tunnel-rs Binary',
      });
      if (selected && typeof selected === 'string') {
        await invoke('set_binary_path', { path: selected });
        setBinaryPath(selected);
      }
    } catch (e) {
      console.error('Failed to set binary path:', e);
      throw e;
    }
  }, []);

  const clearBinaryPath = useCallback(async () => {
    try {
      await invoke('set_binary_path', { path: null });
      setBinaryPath(null);
    } catch (e) {
      console.error('Failed to clear binary path:', e);
      throw e;
    }
  }, []);

  return { binaryPath, loading, refresh, selectBinaryPath, clearBinaryPath };
}
