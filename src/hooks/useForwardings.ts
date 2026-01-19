// React hooks for forwarding management via Tauri

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Forwarding, ForwardingFormData } from '../types';

export function useForwardings() {
    const [forwardings, setForwardings] = useState<Forwarding[]>([]);
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
            mutationCountRef.current = 0;
            setMutating(false);
        }
    }, []);

    const refresh = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);
            const result = await invoke<Forwarding[]>('list_forwardings');
            setForwardings(result);
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        refresh();
    }, [refresh]);

    const createForwarding = useCallback(async (serverGroupId: string, form: ForwardingFormData): Promise<Forwarding> => {
        startMutation();
        try {
            const forwarding = await invoke<Forwarding>('create_forwarding', {
                serverGroupId,
                name: form.name,
                source: form.source,
                target: form.target,
            });

            setError(null);
            await refresh();
            return forwarding;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to create forwarding: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const updateForwarding = useCallback(async (id: string, serverGroupId: string, form: ForwardingFormData): Promise<Forwarding> => {
        startMutation();
        try {
            const forwarding = await invoke<Forwarding>('update_forwarding', {
                id,
                serverGroupId,
                name: form.name,
                source: form.source,
                target: form.target,
            });

            setError(null);
            await refresh();
            return forwarding;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to update forwarding: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const deleteForwarding = useCallback(async (id: string): Promise<void> => {
        startMutation();
        try {
            await invoke('delete_forwarding', { id });
            setError(null);
            await refresh();
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            setError(`Failed to delete forwarding: ${message}`);
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const getForwarding = useCallback((id: string): Forwarding | undefined => {
        return forwardings.find(f => f.id === id);
    }, [forwardings]);

    const getForwardingsByGroup = useCallback((serverGroupId: string): Forwarding[] => {
        return forwardings.filter(f => f.server_group_id === serverGroupId);
    }, [forwardings]);

    return {
        forwardings,
        loading,
        mutating,
        error,
        refresh,
        createForwarding,
        updateForwarding,
        deleteForwarding,
        getForwarding,
        getForwardingsByGroup,
    };
}
