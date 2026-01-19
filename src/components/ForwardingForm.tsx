// Forwarding edit/create form component

import { useState, useEffect } from 'react';
import type { ForwardingFormData } from '../types';
import './ForwardingForm.css';

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
        if (!form.source.trim()) {
            setError('Source address is required');
            return;
        }
        if (!form.target.trim()) {
            setError('Local target is required');
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
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck="false"
                        />
                        <span className="help-text">Remote service to tunnel</span>
                    </div>

                    <div className="form-group">
                        <label htmlFor="target">Local Target *</label>
                        <input
                            id="target"
                            type="text"
                            value={form.target}
                            onChange={handleChange('target')}
                            placeholder="127.0.0.1:2222"
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck="false"
                        />
                        <span className="help-text">Local address to listen on</span>
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
