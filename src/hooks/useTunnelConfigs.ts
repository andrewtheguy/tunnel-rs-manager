// React hooks for tunnel config management via Tauri

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StoredConfig, ConfigFormData } from '../types';

/** Parse comma-separated relay URLs, returning null if empty */
function parseRelayUrls(input: string): string[] | null {
    const urls = input
        .split(',')
        .map(s => s.trim())
        .filter(s => s.length > 0);
    return urls.length > 0 ? urls : null;
}

export function useTunnelConfigs() {
    const [configs, setConfigs] = useState<StoredConfig[]>([]);
    const [loading, setLoading] = useState(true);
    const [mutating, setMutating] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mutationCountRef = useRef(0);

    const startMutation = useCallback(() => {
        mutationCountRef.current += 1;
        setMutating(true);
    }, []);

    const endMutation = useCallback(() => {
        mutationCountRef.current -= 1;
        if (mutationCountRef.current <= 0) {
            mutationCountRef.current = 0; // Clamp to prevent negative
            setMutating(false);
        }
    }, []);

    const refresh = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);
            const result = await invoke<StoredConfig[]>('list_configs');
            setConfigs(result);
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        refresh();
    }, [refresh]);

    const createConfig = useCallback(async (form: ConfigFormData): Promise<StoredConfig> => {
        startMutation();
        try {
            const config = await invoke<StoredConfig>('create_config', {
                name: form.name,
                serverNodeId: form.server_node_id,
                source: form.source || null,
                target: form.target || null,
                authToken: form.auth_token || null,
                relayUrls: parseRelayUrls(form.relay_urls),
            });

            setError(null);
            await refresh();
            return config;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to create config: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const updateConfig = useCallback(async (id: string, form: ConfigFormData): Promise<StoredConfig> => {
        startMutation();
        try {
            const config = await invoke<StoredConfig>('update_config', {
                id,
                name: form.name,
                serverNodeId: form.server_node_id,
                source: form.source || null,
                target: form.target || null,
                authToken: form.auth_token || null,
                relayUrls: parseRelayUrls(form.relay_urls),
            });

            setError(null);
            await refresh();
            return config;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to update config: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const deleteConfig = useCallback(async (id: string): Promise<void> => {
        startMutation();
        try {
            await invoke('delete_config', { id });
            setError(null);
            await refresh();
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to delete config: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    return {
        configs,
        loading,
        mutating,
        error,
        refresh,
        createConfig,
        updateConfig,
        deleteConfig,
    };
}
