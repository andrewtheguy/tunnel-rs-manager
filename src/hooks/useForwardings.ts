// React hooks for forwarding management via Tauri

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Forwarding, ForwardingFormData } from '../types';

function buildForwardingPayload(form: ForwardingFormData) {
    const source = form.source?.trim();
    const target = form.target?.trim();

    if (!source) {
        throw new Error('Source address is required');
    }

    if (!target) {
        throw new Error('Target address is required');
    }

    return {
        name: form.name?.trim() ?? '',
        source,
        target,
    };
}

export function useForwardings() {
    const [forwardings, setForwardings] = useState<Forwarding[]>([]);
    const [loading, setLoading] = useState(true);
    const [mutating, setMutating] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mutationCountRef = useRef(0);
    const mountedRef = useRef(false);

    const startMutation = useCallback(() => {
        mutationCountRef.current += 1;
        if (mountedRef.current) {
            setMutating(true);
        }
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
            if (mountedRef.current) {
                setLoading(true);
                setError(null);
            }
            const result = await invoke<Forwarding[]>('list_forwardings');
            if (!mountedRef.current) return;
            setForwardings(result);
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

    const createForwarding = useCallback(async (serverGroupId: string, form: ForwardingFormData): Promise<Forwarding> => {
        startMutation();
        try {
            const payload = buildForwardingPayload(form);
            const forwarding = await invoke<Forwarding>('create_forwarding', {
                serverGroupId,
                ...payload,
            });

            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
            return forwarding;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to create forwarding: ${message}`);
            }
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const updateForwarding = useCallback(async (id: string, serverGroupId: string, form: ForwardingFormData): Promise<Forwarding> => {
        startMutation();
        try {
            const payload = buildForwardingPayload(form);
            const forwarding = await invoke<Forwarding>('update_forwarding', {
                id,
                serverGroupId,
                ...payload,
            });

            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
            return forwarding;
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to update forwarding: ${message}`);
            }
            throw e;
        } finally {
            endMutation();
        }
    }, [refresh, startMutation, endMutation]);

    const deleteForwarding = useCallback(async (id: string): Promise<void> => {
        startMutation();
        try {
            await invoke('delete_forwarding', { id });
            if (mountedRef.current) {
                setError(null);
                await refresh();
            }
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            if (mountedRef.current) {
                setError(`Failed to delete forwarding: ${message}`);
            }
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
