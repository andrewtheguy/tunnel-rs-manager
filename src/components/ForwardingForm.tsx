// Forwarding edit/create form component

import { useState, useEffect, useMemo } from 'react';
import type { ForwardingFormData } from '../types';
import './ForwardingForm.css';

// Valid hostname: letters, digits, hyphens, dots (no leading/trailing dot or hyphen per label)
const HOST_RE = /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)*[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?$/;
// Valid IPv4
const IPV4_RE = /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/;
// Valid IPv6 in brackets
const IPV6_BRACKET_RE = /^\[[\da-fA-F:]+\]$/;

function isValidHost(host: string): boolean {
    return HOST_RE.test(host) || IPV4_RE.test(host) || IPV6_BRACKET_RE.test(host);
}

function isValidPort(port: string): boolean {
    const n = Number(port);
    return /^\d+$/.test(port) && n >= 1 && n <= 65535;
}

function validateSource(value: string): string | null {
    if (!value) return 'Source address is required';
    const match = value.match(/^(tcp|udp):\/\/(.+):(\d+)$/);
    if (!match) return 'Must be tcp://host:port or udp://host:port';
    const [, , host, port] = match;
    if (!isValidHost(host)) return `Invalid host: ${host}`;
    if (!isValidPort(port)) return `Invalid port: ${port}`;
    return null;
}

function validateTarget(value: string): string | null {
    if (!value) return 'Local target is required';
    const match = value.match(/^(.+):(\d+)$/);
    if (!match) return 'Must be host:port';
    const [, host, port] = match;
    if (!isValidHost(host)) return `Invalid host: ${host}`;
    if (!isValidPort(port)) return `Invalid port: ${port}`;
    return null;
}

interface ForwardingFormProps {
    initial?: ForwardingFormData;
    serverGroupName: string;
    onSubmit: (data: ForwardingFormData) => Promise<void>;
    onCancel: () => void;
    isEditing?: boolean;
}

export function ForwardingForm({ initial, serverGroupName, onSubmit, onCancel, isEditing = false }: ForwardingFormProps) {
    const [form, setForm] = useState<ForwardingFormData>(initial || {
        name: '',
        source: '',
        target: '',
    });
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const sourceError = useMemo(() => validateSource(form.source.trim()), [form.source]);
    const targetError = useMemo(() => validateTarget(form.target.trim()), [form.target]);

    useEffect(() => {
        if (initial) {
            setForm(initial);
        }
    }, [initial]);

    const handleChange = (field: keyof ForwardingFormData) => (
        e: React.ChangeEvent<HTMLInputElement>
    ) => {
        setForm(prev => ({ ...prev, [field]: e.target.value }));
        setError(null);
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        // Validation
        if (!form.name.trim()) {
            setError('Name is required');
            return;
        }
        if (sourceError) {
            setError(sourceError);
            return;
        }
        if (targetError) {
            setError(targetError);
            return;
        }

        setSubmitting(true);
        setError(null);
        try {
            await onSubmit(form);
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <form className="forwarding-form" onSubmit={handleSubmit}>
            <h2 className="form-title">
                {isEditing ? 'Edit Forwarding' : 'New Forwarding'}
            </h2>
            <p className="form-subtitle">Server Group: {serverGroupName}</p>

            {error && <div className="form-error">{error}</div>}

            <fieldset disabled={submitting}>
                <div className="form-group">
                    <label htmlFor="name">Name *</label>
                    <input
                        id="name"
                        type="text"
                        value={form.name}
                        onChange={handleChange('name')}
                        placeholder="SSH Tunnel"
                        autoFocus
                        autoCapitalize="none"
                    />
                </div>

                <div className="form-row">
                    <div className="form-group">
                        <label htmlFor="source">Source Address *</label>
                        <input
                            id="source"
                            type="text"
                            value={form.source}
                            onChange={handleChange('source')}
                            placeholder="tcp://127.0.0.1:22"
                            className={sourceError && form.source.trim() ? 'input-error' : ''}
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck="false"
                        />
                        {sourceError && form.source.trim() ? (
                            <span className="field-error">{sourceError}</span>
                        ) : (
                            <span className="help-text">tcp://host:port or udp://host:port</span>
                        )}
                    </div>

                    <div className="form-group">
                        <label htmlFor="target">Local Target *</label>
                        <input
                            id="target"
                            type="text"
                            value={form.target}
                            onChange={handleChange('target')}
                            placeholder="localhost:2222"
                            className={targetError && form.target.trim() ? 'input-error' : ''}
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck="false"
                        />
                        {targetError && form.target.trim() ? (
                            <span className="field-error">{targetError}</span>
                        ) : (
                            <span className="help-text">host:port</span>
                        )}
                    </div>
                </div>

                <div className="form-actions">
                    <button type="button" className="btn-secondary" onClick={onCancel}>
                        Cancel
                    </button>
                    <button type="submit" className="btn-primary">
                        {submitting ? 'Saving...' : isEditing ? 'Save Changes' : 'Create'}
                    </button>
                </div>
            </fieldset>
        </form>
    );
}
