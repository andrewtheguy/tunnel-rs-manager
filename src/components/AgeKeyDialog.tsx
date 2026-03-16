import { useState, useEffect } from 'react';
import './AgeKeyDialog.css';

interface AgeKeyDialogProps {
    mode: 'setup' | 'select';
    recipients?: string[];
    onComplete: (recipient: string) => void;
    onCancel: () => void;
    onGenerate: () => Promise<string>;
    loading?: boolean;
    error?: string;
}

export function AgeKeyDialog({ mode, recipients = [], onComplete, onCancel, onGenerate, loading = false, error }: AgeKeyDialogProps) {
    const [generatedKey, setGeneratedKey] = useState<string | null>(null);
    const [generating, setGenerating] = useState(false);
    const [selectedRecipient, setSelectedRecipient] = useState<string>(recipients[0] ?? '');
    const [genError, setGenError] = useState<string | undefined>();

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape' && !loading && !generating) {
                onCancel();
            }
        };
        document.addEventListener('keydown', handleKeyDown);
        return () => document.removeEventListener('keydown', handleKeyDown);
    }, [onCancel, loading, generating]);

    useEffect(() => {
        if (recipients.length > 0 && (!selectedRecipient || !recipients.includes(selectedRecipient))) {
            setSelectedRecipient(recipients[0]);
        }
    }, [recipients, selectedRecipient]);

    const handleGenerate = async () => {
        setGenerating(true);
        setGenError(undefined);
        try {
            const pubKey = await onGenerate();
            setGeneratedKey(pubKey);
        } catch (e) {
            setGenError(e instanceof Error ? e.message : String(e));
        } finally {
            setGenerating(false);
        }
    };

    const handleContinue = () => {
        if (generatedKey) {
            onComplete(generatedKey);
        }
    };

    const handleSelect = () => {
        if (selectedRecipient) {
            onComplete(selectedRecipient);
        }
    };

    return (
        <div className="confirm-overlay" onClick={loading || generating ? undefined : onCancel}>
            <div
                className="age-key-dialog"
                role="dialog"
                aria-modal="true"
                onClick={e => e.stopPropagation()}
            >
                {mode === 'setup' && !generatedKey && (
                    <>
                        <h3>No Age Encryption Key Found</h3>
                        <p className="age-key-description">
                            An age encryption key is needed to encrypt credentials for export.
                            A new keypair will be generated and saved locally.
                        </p>

                        {genError && <p className="age-key-error">{genError}</p>}
                        {error && <p className="age-key-error">{error}</p>}

                        <div className="age-key-actions">
                            <button
                                type="button"
                                className="btn btn-secondary"
                                onClick={onCancel}
                                disabled={generating}
                            >
                                Cancel
                            </button>
                            <button
                                type="button"
                                className="btn btn-primary"
                                onClick={handleGenerate}
                                disabled={generating}
                            >
                                {generating ? 'Generating...' : 'Generate Key'}
                            </button>
                        </div>
                    </>
                )}

                {mode === 'setup' && generatedKey && (
                    <>
                        <h3>Key Generated</h3>
                        <p className="age-key-description">
                            Your new age public key:
                        </p>
                        <code className="age-key-pubkey">{generatedKey}</code>

                        <div className="age-key-actions">
                            <button
                                type="button"
                                className="btn btn-primary"
                                onClick={handleContinue}
                                disabled={loading}
                            >
                                {loading ? 'Processing...' : 'Continue'}
                            </button>
                        </div>
                    </>
                )}

                {mode === 'select' && (
                    <>
                        <h3>Select Encryption Key</h3>
                        <p className="age-key-description">
                            Multiple keys found. Select which key to use for encryption:
                        </p>

                        <div className="age-key-list">
                            {recipients.map((r) => (
                                <label key={r} className="age-key-option">
                                    <input
                                        type="radio"
                                        name="recipient"
                                        value={r}
                                        checked={selectedRecipient === r}
                                        onChange={() => setSelectedRecipient(r)}
                                    />
                                    <code>{r}</code>
                                </label>
                            ))}
                        </div>

                        {error && <p className="age-key-error">{error}</p>}

                        <div className="age-key-actions">
                            <button
                                type="button"
                                className="btn btn-secondary"
                                onClick={onCancel}
                                disabled={loading}
                            >
                                Cancel
                            </button>
                            <button
                                type="button"
                                className="btn btn-primary"
                                onClick={handleSelect}
                                disabled={!selectedRecipient || loading}
                            >
                                {loading ? 'Processing...' : 'Use Selected'}
                            </button>
                        </div>
                    </>
                )}
            </div>
        </div>
    );
}
