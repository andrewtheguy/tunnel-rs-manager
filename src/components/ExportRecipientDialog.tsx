import { useState, useEffect, useCallback } from 'react';
import './PassphraseDialog.css';

interface ExportRecipientDialogProps {
  forwardingName: string;
  initialRecipient: string;
  /** Resolves once the export (or unlock hand-off) succeeds; rejects to show an error. */
  onExport: (recipient: string) => Promise<void>;
  onCancel: () => void;
}

export function ExportRecipientDialog({
  forwardingName,
  initialRecipient,
  onExport,
  onCancel,
}: ExportRecipientDialogProps) {
  const [recipient, setRecipient] = useState(initialRecipient);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !loading) {
        onCancel();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onCancel, loading]);

  const submit = useCallback(
    async (value: string) => {
      setError(null);
      setLoading(true);
      try {
        await onExport(value);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [onExport],
  );

  const trimmed = recipient.trim();

  return (
    <div className="confirm-overlay">
      <div
        className="passphrase-dialog"
        role="dialog"
        aria-modal="true"
        onClick={e => e.stopPropagation()}
      >
        <form
          onSubmit={e => {
            e.preventDefault();
            submit(trimmed);
          }}
        >
          <h3>Export config</h3>
          <p className="passphrase-description">
            Exporting <strong>{forwardingName}</strong> as a tunnel-rs config. Enter an age
            recipient (public key, <code>age1…</code>) to include the auth and ALPN tokens
            encrypted to that key. Leave it blank to export placeholder values only.
          </p>

          <label className="passphrase-field">
            <span>Age recipient</span>
            <input
              type="text"
              value={recipient}
              onChange={e => setRecipient(e.target.value)}
              placeholder="age1... (optional)"
              disabled={loading}
              autoFocus
              spellCheck={false}
              autoCapitalize="none"
            />
          </label>

          {error && <p className="passphrase-error">{error}</p>}

          <div className="passphrase-actions">
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
              disabled={loading}
            >
              {loading
                ? 'Exporting...'
                : trimmed
                  ? 'Export encrypted'
                  : 'Export without encryption'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
