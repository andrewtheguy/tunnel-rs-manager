// Server Group edit/create form component

import { useState, useEffect, useMemo, useRef } from 'react';
import type { ServerGroupFormData } from '../types';
import './ServerGroupForm.css';

// Base64URL character set (A-Z, a-z, 0-9, -, _)
const BASE64URL_REGEX = /^[A-Za-z0-9_-]+$/;

/** CRC16-CCITT-FALSE: poly=0x1021, init=0xFFFF, no reflection, no XOR-out */
function crc16CcittFalse(data: Uint8Array): number {
    let crc = 0xFFFF;
    for (const byte of data) {
        crc ^= byte << 8;
        for (let i = 0; i < 8; i++) {
            if (crc & 0x8000) {
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF;
            } else {
                crc = (crc << 1) & 0xFFFF;
            }
        }
    }
    return crc;
}

function base64UrlDecode(s: string): Uint8Array | null {
    let base64 = s.replace(/-/g, '+').replace(/_/g, '/');
    while (base64.length % 4 !== 0) {
        base64 += '=';
    }
    try {
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes;
    } catch {
        return null;
    }
}

/** Validate a Base64URL-encoded payload with trailing 2-byte CRC16 big-endian checksum */
function validateBase64UrlCrc16(base64Payload: string, label: string): string | null {
    if (!BASE64URL_REGEX.test(base64Payload)) {
        return `${label} contains invalid characters (only A-Z, a-z, 0-9, -, _ allowed)`;
    }

    const decoded = base64UrlDecode(base64Payload);
    if (!decoded || decoded.length < 3) {
        return `${label} is not valid Base64URL`;
    }

    const randomBytes = decoded.slice(0, decoded.length - 2);
    const storedCrc = (decoded[decoded.length - 2] << 8) | decoded[decoded.length - 1];
    const computedCrc = crc16CcittFalse(randomBytes);

    if (storedCrc !== computedCrc) {
        return `Invalid ${label.toLowerCase()} checksum`;
    }

    return null;
}

/** Auth token: 'i' prefix + 46-char Base64URL(32 random bytes + 2-byte CRC16 BE) = 47 chars */
function validateAuthToken(token: string): string | null {
    if (!token) {
        return 'Auth token is required';
    }
    if (token.length !== 47) {
        return `Auth token must be exactly 47 characters (got ${token.length})`;
    }
    if (token[0] !== 'i') {
        return "Auth token must start with 'i'";
    }
    return validateBase64UrlCrc16(token.slice(1), 'Auth token');
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
    const appliedInitialRef = useRef(false);
    const normalizedAuthToken = form.auth_token.trim();

    const authTokenError = useMemo(
        () => validateAuthToken(normalizedAuthToken),
        [normalizedAuthToken]
    );

    useEffect(() => {
        if (initial && !appliedInitialRef.current) {
            setForm(initial);
            appliedInitialRef.current = true;
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

        // Create trimmed copy for validation and submission
        const trimmedForm: ServerGroupFormData = {
            name: form.name.trim(),
            server_node_id: form.server_node_id.trim(),
            auth_token: form.auth_token.trim(),
            relay_urls: form.relay_urls.trim(),
        };

        // Validation
        if (!trimmedForm.name) {
            setError('Name is required');
            return;
        }
        if (!trimmedForm.server_node_id) {
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
            await onSubmit(trimmedForm);
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
                        autoCapitalize="none"
                        autoCorrect="off"
                        spellCheck={false}
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
                        autoCapitalize="none"
                        autoComplete="off"
                        autoCorrect="off"
                        spellCheck={false}
                    />
                    <span className="help-text">The EndpointId of the tunnel-rs server</span>
                </div>

                <div className="form-group">
                    <label htmlFor="auth_token">Auth Token *</label>
                    <input
                        id="auth_token"
                        type="password"
                        value={form.auth_token}
                        onChange={handleChange('auth_token')}
                        placeholder="XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX1234"
                        className={authTokenError && normalizedAuthToken ? 'input-error' : ''}
                        autoCapitalize="none"
                        autoComplete="off"
                        autoCorrect="off"
                        spellCheck={false}
                    />
                    {authTokenError && normalizedAuthToken ? (
                        <span className="field-error">{authTokenError}</span>
                    ) : (
                        <span className="help-text">47-character token from server admin</span>
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
                        autoCapitalize="none"
                        autoComplete="off"
                        autoCorrect="off"
                        spellCheck={false}
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
