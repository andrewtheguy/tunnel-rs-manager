// React hooks for tunnel instance management via Tauri

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { TunnelInstance } from '../types';

export function useTunnelInstances() {
    const [instances, setInstances] = useState<TunnelInstance[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const pollingRef = useRef<number | null>(null);

    const refresh = useCallback(async () => {
        try {
            const result = await invoke<TunnelInstance[]>('list_instances');
            setInstances(result);
            setError(null);
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        }
    }, []);

    // Start polling when component mounts
    useEffect(() => {
        refresh();
        pollingRef.current = window.setInterval(refresh, 1000);
        return () => {
            if (pollingRef.current) {
                window.clearInterval(pollingRef.current);
            }
        };
    }, [refresh]);

    const startTunnel = useCallback(async (id: string): Promise<void> => {
        setLoading(true);
        try {
            await invoke('start_tunnel', { id });
            await refresh();
        } finally {
            setLoading(false);
        }
    }, [refresh]);

    const stopTunnel = useCallback(async (id: string): Promise<void> => {
        setLoading(true);
        try {
            await invoke('stop_tunnel', { id });
            await refresh();
        } finally {
            setLoading(false);
        }
    }, [refresh]);

    const getInstance = useCallback((configId: string): TunnelInstance | undefined => {
        return instances.find(i => i.config_id === configId);
    }, [instances]);

    return {
        instances,
        loading,
        error,
        refresh,
        startTunnel,
        stopTunnel,
        getInstance,
    };
}
