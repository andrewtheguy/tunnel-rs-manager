import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function useBinaryPath() {
  const [customBinaryPath, setCustomBinaryPath] = useState<string | null>(null);
  const [isUsingBundled, setIsUsingBundled] = useState(true);
  const [binaryVersion, setBinaryVersion] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [path, bundled, version] = await Promise.all([
        invoke<string | null>('get_custom_binary_path'),
        invoke<boolean>('is_using_bundled_binary'),
        invoke<string>('get_binary_version').catch(() => null),
      ]);
      setCustomBinaryPath(path);
      setIsUsingBundled(bundled);
      setBinaryVersion(version);
    } catch (e) {
      console.error('Failed to get binary path:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const selectCustomBinaryPath = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: 'Select tunnel-rs Binary',
      });
      if (selected && typeof selected === 'string') {
        await invoke('set_custom_binary_path', { path: selected });
        setCustomBinaryPath(selected);
        setIsUsingBundled(false);
      }
    } catch (e) {
      console.error('Failed to set custom binary path:', e);
      throw e;
    }
  }, []);

  const useBundledBinary = useCallback(async () => {
    try {
      await invoke('set_custom_binary_path', { path: null });
      setCustomBinaryPath(null);
      setIsUsingBundled(true);
    } catch (e) {
      console.error('Failed to switch to bundled binary:', e);
      throw e;
    }
  }, []);

  return {
    customBinaryPath,
    isUsingBundled,
    binaryVersion,
    loading,
    refresh,
    selectCustomBinaryPath,
    useBundledBinary,
  };
}
