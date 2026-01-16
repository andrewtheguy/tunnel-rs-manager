// React hooks for tunnel config management via Tauri

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StoredConfig, ConfigFormData } from '../types';

export function useTunnelConfigs() {
    const [configs, setConfigs] = useState<StoredConfig[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

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
        const relay_urls = form.relay_urls
            .split(',')
            .map(s => s.trim())
            .filter(s => s.length > 0);

        const config = await invoke<StoredConfig>('create_config', {
            name: form.name,
            serverNodeId: form.server_node_id,
            source: form.source || null,
            target: form.target || null,
            authToken: form.auth_token || null,
            relayUrls: relay_urls.length > 0 ? relay_urls : null,
        });

        await refresh();
        return config;
    }, [refresh]);

    const updateConfig = useCallback(async (id: string, form: ConfigFormData): Promise<StoredConfig> => {
        const relay_urls = form.relay_urls
            .split(',')
            .map(s => s.trim())
            .filter(s => s.length > 0);

        const config = await invoke<StoredConfig>('update_config', {
            id,
            name: form.name,
            serverNodeId: form.server_node_id,
            source: form.source || null,
            target: form.target || null,
            authToken: form.auth_token || null,
            relayUrls: relay_urls.length > 0 ? relay_urls : null,
        });

        await refresh();
        return config;
    }, [refresh]);

    const deleteConfig = useCallback(async (id: string): Promise<void> => {
        await invoke('delete_config', { id });
        await refresh();
    }, [refresh]);

    return {
        configs,
        loading,
        error,
        refresh,
        createConfig,
        updateConfig,
        deleteConfig,
    };
}
