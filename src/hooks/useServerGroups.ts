// React hooks for server group management via Tauri

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ServerGroup, ServerGroupFormData } from '../types';

/** Parse comma-separated relay URLs, returning empty array if empty */
function parseRelayUrls(input: string): string[] {
    const urls = input
        .split(',')
        .map(s => s.trim())
        .filter(s => s.length > 0);
    return urls;
}

export function useServerGroups() {
    const [serverGroups, setServerGroups] = useState<ServerGroup[]>([]);
    const [loading, setLoading] = useState(true);
    const [mutating, setMutating] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mutationCountRef = useRef(0);
    const mountedRef = useRef(true);

    const startMutation = useCallback(() => {
        mutationCountRef.current += 1;
        setMutating(true);
    }, []);

    const endMutation = useCallback(() => {
        mutationCountRef.current -= 1;
        if (mutationCountRef.current <= 0) {
            mutationCountRef.current = 0;
            if (mountedRef.current) {
                setMutating(false);
            }
        }
    }, []);

    const refresh = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);
            const result = await invoke<ServerGroup[]>('list_server_groups');
            if (!mountedRef.current) return;
            setServerGroups(result);
        } catch (e) {
            if (!mountedRef.current) return;
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            if (mountedRef.current) {
                setLoading(false);
            }
        }
    }, []);

    useEffect(() => {
        mountedRef.current = true;
        refresh();
        return () => {
            mountedRef.current = false;
        };
    }, [refresh]);

    const createServerGroup = useCallback(async (form: ServerGroupFormData): Promise<ServerGroup> => {
        startMutation();
        try {
            const group = await invoke<ServerGroup>('create_server_group', {
                name: form.name,
                serverNodeId: form.server_node_id,
                authToken: form.auth_token,
                relayUrls: parseRelayUrls(form.relay_urls),
            });

            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
            return group;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to create server group: ${message}`);
            }
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const updateServerGroup = useCallback(async (id: string, form: ServerGroupFormData): Promise<ServerGroup> => {
        startMutation();
        try {
            const group = await invoke<ServerGroup>('update_server_group', {
                id,
                name: form.name,
                serverNodeId: form.server_node_id,
                authToken: form.auth_token,
                relayUrls: parseRelayUrls(form.relay_urls),
            });

            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
            return group;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to update server group: ${message}`);
            }
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const deleteServerGroup = useCallback(async (id: string): Promise<void> => {
        startMutation();
        try {
            await invoke('delete_server_group', { id });
            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to delete server group: ${message}`);
            }
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const getServerGroup = useCallback((id: string): ServerGroup | undefined => {
        return serverGroups.find(g => g.id === id);
    }, [serverGroups]);

    return {
        serverGroups,
        loading,
        mutating,
        error,
        refresh,
        createServerGroup,
        updateServerGroup,
        deleteServerGroup,
        getServerGroup,
    };
}
