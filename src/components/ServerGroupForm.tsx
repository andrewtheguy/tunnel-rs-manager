// Server Group edit/create form component

import { useState, useEffect, useMemo } from 'react';
import type { ServerGroupFormData } from '../types';
import './ServerGroupForm.css';

// Valid characters for auth token body: A-Za-z0-9 and -_.
const TOKEN_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.';

function charToIndex(c: string): number {
    const idx = TOKEN_ALPHABET.indexOf(c);
    return idx;
}

function luhnModNChecksum(body: string): string {
    const n = TOKEN_ALPHABET.length; // 65
    let factor = 2;
    let sum = 0;

    // Process characters from right to left
    for (let i = body.length - 1; i >= 0; i--) {
        const codePoint = charToIndex(body[i]);
        if (codePoint === -1) {
            return ''; // Invalid character
        }
        let addend = factor * codePoint;
        factor = factor === 2 ? 1 : 2;
        addend = Math.floor(addend / n) + (addend % n);
        sum += addend;
    }

    const remainder = sum % n;
    const checkCodePoint = (n - remainder) % n;
    return TOKEN_ALPHABET[checkCodePoint];
}

function validateAuthToken(token: string): string | null {
    // Empty token is valid (optional field)
    if (!token) {
        return null;
    }

    // Must be exactly 18 characters
    if (token.length !== 18) {
        return `Token must be exactly 18 characters (got ${token.length})`;
    }

    // Must start with 'i'
    if (token[0] !== 'i') {
        return "Token must start with 'i'";
    }

    // Body is characters 1-16 (indices 1..17)
    const body = token.slice(1, 17);

    // Validate body characters
    for (const c of body) {
        if (charToIndex(c) === -1) {
            return `Invalid character '${c}' in token body`;
        }
    }

    // Validate checksum (last character)
    const expectedChecksum = luhnModNChecksum(body);
    if (token[17] !== expectedChecksum) {
        return 'Invalid token checksum';
    }

    return null;
}

interface ServerGroupFormProps {
    initial?: ServerGroupFormData;
    onSubmit: (data: ServerGroupFormData) => Promise<void>;
    onCancel: () => void;
    isEditing?: boolean;
}

export function ServerGroupForm({ initial, onSubmit, onCancel, isEditing = false }: ServerGroupFormProps) {
    const [form, setForm] = useState<ServerGroupFormData>(initial || {
        name: '',
        server_node_id: '',
        auth_token: '',
        relay_urls: '',
    });
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const authTokenError = useMemo(
        () => validateAuthToken(form.auth_token),
        [form.auth_token]
    );

    useEffect(() => {
        if (initial) {
            setForm(initial);
        }
    }, [initial]);

    const handleChange = (field: keyof ServerGroupFormData) => (
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
        if (authTokenError) {
            setError(authTokenError);
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
        <form className="server-group-form" onSubmit={handleSubmit}>
            <h2 className="form-title">
                {isEditing ? 'Edit Server Group' : 'New Server Group'}
            </h2>

            {error && <div className="form-error">{error}</div>}

            <fieldset disabled={submitting}>
                <div className="form-group">
                    <label htmlFor="name">Name *</label>
                    <input
                        id="name"
                        type="text"
                        value={form.name}
                        onChange={handleChange('name')}
                        placeholder="My Server"
                        autoFocus
                        autoCapitalize="off"
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
                        autoCapitalize="off"
                    />
                    <span className="help-text">The EndpointId of the tunnel-rs server</span>
                </div>

                <div className="form-group">
                    <label htmlFor="auth_token">Auth Token</label>
                    <input
                        id="auth_token"
                        type="password"
                        value={form.auth_token}
                        onChange={handleChange('auth_token')}
                        placeholder="iXXXXXXXXXXXXXXXXX"
                        className={authTokenError ? 'input-error' : ''}
                        autoCapitalize="off"
                    />
                    {authTokenError ? (
                        <span className="field-error">{authTokenError}</span>
                    ) : (
                        <span className="help-text">18-character token from server admin</span>
                    )}
                </div>

                <div className="form-group">
                    <label htmlFor="relay_urls">Relay URLs (optional)</label>
                    <textarea
                        id="relay_urls"
                        value={form.relay_urls}
                        onChange={handleChange('relay_urls')}
                        placeholder="https://relay1.example.com, https://relay2.example.com"
                        rows={2}
                        autoCapitalize="off"
                    />
                    <span className="help-text">Comma-separated custom relay URLs</span>
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
