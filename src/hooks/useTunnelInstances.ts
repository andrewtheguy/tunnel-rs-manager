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

    // Poll for instance updates; 5s interval balances responsiveness with resource usage.
    // Manual actions (start/stop) call refresh() immediately for instant feedback.
    useEffect(() => {
        refresh();
        pollingRef.current = window.setInterval(refresh, 5000);
        return () => {
            if (pollingRef.current) {
                window.clearInterval(pollingRef.current);
            }
        };
    }, [refresh]);

    const startTunnel = useCallback(async (forwardingId: string): Promise<void> => {
        setLoading(true);
        setError(null);
        try {
            await invoke('start_tunnel', { forwardingId });
            await refresh();
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to start tunnel: ${message}`);
            throw e;
        } finally {
            setLoading(false);
        }
    }, [refresh]);

    const stopTunnel = useCallback(async (forwardingId: string): Promise<void> => {
        setLoading(true);
        setError(null);
        try {
            await invoke('stop_tunnel', { forwardingId });
            await refresh();
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to stop tunnel: ${message}`);
            throw e;
        } finally {
            setLoading(false);
        }
    }, [refresh]);

    const getInstance = useCallback((forwardingId: string): TunnelInstance | undefined => {
        return instances.find(i => i.forwarding_id === forwardingId);
    }, [instances]);

    /** Check if any forwarding in a group is running */
    const isGroupRunning = useCallback((serverGroupId: string, forwardings: { id: string; server_group_id: string }[]): boolean => {
        const groupForwardingIds = forwardings
            .filter(f => f.server_group_id === serverGroupId)
            .map(f => f.id);
        return instances.some(
            i => groupForwardingIds.includes(i.forwarding_id) &&
                 (i.status === 'running' || i.status === 'starting')
        );
    }, [instances]);

    return {
        instances,
        loading,
        error,
        refresh,
        startTunnel,
        stopTunnel,
        getInstance,
        isGroupRunning,
    };
}
