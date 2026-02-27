import { useState, useEffect, useMemo } from 'react';
import './PassphraseDialog.css';

interface PassphraseDialogProps {
    mode: 'export' | 'import';
    onSubmit: (passphrase: string) => void;
    onSkip?: () => void;
    onCancel: () => void;
    loading?: boolean;
    error?: string;
}

export function PassphraseDialog({ mode, onSubmit, onSkip, onCancel, loading = false, error }: PassphraseDialogProps) {
    const [passphrase, setPassphrase] = useState('');
    const [confirm, setConfirm] = useState('');

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape' && !loading) {
                onCancel();
            }
        };
        document.addEventListener('keydown', handleKeyDown);
        return () => document.removeEventListener('keydown', handleKeyDown);
    }, [onCancel, loading]);

    const hints = useMemo(() => ({
        length: passphrase.length >= 12,
        upper: /[A-Z]/.test(passphrase),
        lower: /[a-z]/.test(passphrase),
        symbol: /[^a-zA-Z0-9]/.test(passphrase),
    }), [passphrase]);

    const isValid = mode === 'export'
        ? hints.length && hints.upper && hints.lower && hints.symbol && passphrase === confirm
        : passphrase.length > 0;

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (isValid && !loading) {
            onSubmit(passphrase);
        }
    };

    return (
        <div className="confirm-overlay" onClick={loading ? undefined : onCancel}>
            <div
                className="passphrase-dialog"
                role="dialog"
                aria-modal="true"
                onClick={e => e.stopPropagation()}
            >
                <h3>{mode === 'export' ? 'Encrypt Credentials' : 'Enter Passphrase'}</h3>

                <form onSubmit={handleSubmit}>
                    <div className="passphrase-field">
                        <label>Passphrase</label>
                        <input
                            type="password"
                            value={passphrase}
                            onChange={e => setPassphrase(e.target.value)}
                            autoFocus
                            disabled={loading}
                        />
                    </div>

                    {mode === 'export' && (
                        <>
                            <div className="passphrase-field">
                                <label>Confirm Passphrase</label>
                                <input
                                    type="password"
                                    value={confirm}
                                    onChange={e => setConfirm(e.target.value)}
                                    disabled={loading}
                                />
                            </div>

                            <ul className="passphrase-hints">
                                <li className={hints.length ? 'passphrase-hint-met' : 'passphrase-hint-unmet'}>
                                    {hints.length ? '\u2713' : '\u2717'} At least 12 characters
                                </li>
                                <li className={hints.upper ? 'passphrase-hint-met' : 'passphrase-hint-unmet'}>
                                    {hints.upper ? '\u2713' : '\u2717'} Uppercase letter
                                </li>
                                <li className={hints.lower ? 'passphrase-hint-met' : 'passphrase-hint-unmet'}>
                                    {hints.lower ? '\u2713' : '\u2717'} Lowercase letter
                                </li>
                                <li className={hints.symbol ? 'passphrase-hint-met' : 'passphrase-hint-unmet'}>
                                    {hints.symbol ? '\u2713' : '\u2717'} Symbol
                                </li>
                                {passphrase.length > 0 && confirm.length > 0 && passphrase !== confirm && (
                                    <li className="passphrase-hint-unmet">
                                        {'\u2717'} Passphrases do not match
                                    </li>
                                )}
                            </ul>
                        </>
                    )}

                    {error && <p className="passphrase-error">{error}</p>}

                    <div className="passphrase-actions">
                        {mode === 'export' && onSkip && (
                            <button
                                type="button"
                                className="btn btn-secondary passphrase-skip"
                                onClick={onSkip}
                                disabled={loading}
                            >
                                Export without credentials
                            </button>
                        )}
                        <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={onCancel}
                            disabled={loading}
                        >
                            Cancel
                        </button>
                        <button
                            type="submit"
                            className="btn btn-primary"
                            disabled={!isValid || loading}
                        >
                            {loading ? (mode === 'export' ? 'Encrypting...' : 'Decrypting...') : (mode === 'export' ? 'Export' : 'Import')}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
