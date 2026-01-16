// Config edit/create form component

import { useState, useEffect } from 'react';
import type { ConfigFormData } from '../types';
import './ConfigForm.css';

interface ConfigFormProps {
    initial?: ConfigFormData;
    onSubmit: (data: ConfigFormData) => Promise<void>;
    onCancel: () => void;
    isEditing?: boolean;
}

export function ConfigForm({ initial, onSubmit, onCancel, isEditing = false }: ConfigFormProps) {
    const [form, setForm] = useState<ConfigFormData>(initial || {
        name: '',
        server_node_id: '',
        source: '',
        target: '',
        auth_token: '',
        relay_urls: '',
    });
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (initial) {
            setForm(initial);
        }
    }, [initial]);

    const handleChange = (field: keyof ConfigFormData) => (
        e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
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
        if (!form.server_node_id.trim()) {
            setError('Server Node ID is required');
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
        <form className="config-form" onSubmit={handleSubmit}>
            <h2 className="form-title">
                {isEditing ? 'Edit Configuration' : 'New Configuration'}
            </h2>

            {error && <div className="form-error">{error}</div>}

            <div className="form-group">
                <label htmlFor="name">Name *</label>
                <input
                    id="name"
                    type="text"
                    value={form.name}
                    onChange={handleChange('name')}
                    placeholder="My SSH Tunnel"
                    autoFocus
                />
            </div>

            <div className="form-group">
                <label htmlFor="server_node_id">Server Node ID *</label>
                <input
                    id="server_node_id"
                    type="text"
                    value={form.server_node_id}
                    onChange={handleChange('server_node_id')}
                    placeholder="2xnbkpbc7izsilvewd7c62w7wnwziacmpfwvhcrya5nt76dqkpga"
                />
                <span className="help-text">The EndpointId of the tunnel-rs server</span>
            </div>

            <div className="form-row">
                <div className="form-group">
                    <label htmlFor="source">Source Address</label>
                    <input
                        id="source"
                        type="text"
                        value={form.source}
                        onChange={handleChange('source')}
                        placeholder="tcp://127.0.0.1:22"
                    />
                    <span className="help-text">Remote service to tunnel</span>
                </div>

                <div className="form-group">
                    <label htmlFor="target">Local Target</label>
                    <input
                        id="target"
                        type="text"
                        value={form.target}
                        onChange={handleChange('target')}
                        placeholder="127.0.0.1:2222"
                    />
                    <span className="help-text">Local address to listen on</span>
                </div>
            </div>

            <div className="form-group">
                <label htmlFor="auth_token">Auth Token</label>
                <input
                    id="auth_token"
                    type="password"
                    value={form.auth_token}
                    onChange={handleChange('auth_token')}
                    placeholder="iXXXXXXXXXXXXXXXXX"
                />
                <span className="help-text">18-character token from server admin</span>
            </div>

            <div className="form-group">
                <label htmlFor="relay_urls">Relay URLs (optional)</label>
                <textarea
                    id="relay_urls"
                    value={form.relay_urls}
                    onChange={handleChange('relay_urls')}
                    placeholder="https://relay1.example.com, https://relay2.example.com"
                    rows={2}
                />
                <span className="help-text">Comma-separated custom relay URLs</span>
            </div>

            <div className="form-actions">
                <button type="button" className="btn-secondary" onClick={onCancel} disabled={submitting}>
                    Cancel
                </button>
                <button type="submit" className="btn-primary" disabled={submitting}>
                    {submitting ? 'Saving...' : isEditing ? 'Save Changes' : 'Create'}
                </button>
            </div>
        </form>
    );
}
