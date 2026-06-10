import { useState, useEffect, useCallback } from 'react';
import './PassphraseDialog.css';

interface SourcePassphraseDialogProps {
  error: string | null;
  /** Resolves when the import retry finishes; the parent reopens this dialog on a wrong passphrase. */
  onSubmit: (passphrase: string) => Promise<void> | void;
  onCancel: () => void;
}

export function SourcePassphraseDialog({ error, onSubmit, onCancel }: SourcePassphraseDialogProps) {
  const [passphrase, setPassphrase] = useState('');
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

  const submit = useCallback(async () => {
    if (!passphrase) return;
    setLoading(true);
    try {
      await onSubmit(passphrase);
    } finally {
      setLoading(false);
    }
  }, [passphrase, onSubmit]);

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
            submit();
          }}
        >
          <h3>Source passphrase</h3>
          <p className="passphrase-description">
            This import contains encrypted secrets from another instance. Enter the passphrase
            they were originally encrypted with so they can be re-encrypted for this instance.
          </p>

          <label className="passphrase-field">
            <span>Original passphrase</span>
            <input
              type="password"
              value={passphrase}
              onChange={e => setPassphrase(e.target.value)}
              placeholder="Passphrase used when exported"
              disabled={loading}
              autoFocus
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
            <button type="submit" className="btn btn-primary" disabled={loading || !passphrase}>
              {loading ? 'Importing...' : 'Import'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
