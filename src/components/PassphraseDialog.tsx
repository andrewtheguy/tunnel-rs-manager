import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './PassphraseDialog.css';

interface PassphraseDialogProps {
  mode: 'setup' | 'unlock';
  onComplete: () => void;
  onCancel: () => void;
}

const MIN_PASSPHRASE_LEN = 12;

export function PassphraseDialog({ mode, onComplete, onCancel }: PassphraseDialogProps) {
  const [instance, setInstance] = useState('default');
  const [passphrase, setPassphrase] = useState('');
  const [confirm, setConfirm] = useState('');
  const [saveToKeychain, setSaveToKeychain] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const clearSensitive = useCallback(() => {
    setPassphrase('');
    setConfirm('');
  }, []);

  useEffect(() => {
    return () => clearSensitive();
  }, [clearSensitive]);

  const handleComplete = useCallback(() => {
    clearSensitive();
    onComplete();
  }, [clearSensitive, onComplete]);

  const handleCancel = useCallback(() => {
    clearSensitive();
    setError(null);
    onCancel();
  }, [clearSensitive, onCancel]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !loading) {
        handleCancel();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleCancel, loading]);

  const handleSetup = async () => {
    setError(null);
    if (passphrase.length < MIN_PASSPHRASE_LEN) {
      setError(`Passphrase must be at least ${MIN_PASSPHRASE_LEN} characters.`);
      return;
    }
    if (!/[a-z]/.test(passphrase)) {
      setError('Passphrase must contain a lowercase letter.');
      return;
    }
    if (!/[A-Z]/.test(passphrase)) {
      setError('Passphrase must contain an uppercase letter.');
      return;
    }
    if (!/[0-9]/.test(passphrase)) {
      setError('Passphrase must contain a digit.');
      return;
    }
    if (!/[^a-zA-Z0-9]/.test(passphrase)) {
      setError('Passphrase must contain a special character.');
      return;
    }
    if (passphrase !== confirm) {
      setError('Passphrases do not match.');
      return;
    }
    if (!instance.trim()) {
      setError('Instance name is required.');
      return;
    }
    setLoading(true);
    try {
      await invoke('setup_passphrase', {
        instance: instance.trim(),
        passphrase,
        saveToKeychain,
      });
      handleComplete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleUnlock = async () => {
    setError(null);
    if (!passphrase) {
      setError('Please enter your passphrase.');
      return;
    }
    setLoading(true);
    try {
      await invoke('unlock_passphrase', {
        passphrase,
        saveToKeychain,
      });
      handleComplete();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('Wrong passphrase')) {
        setError('Wrong passphrase. Please try again.');
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (mode === 'setup') {
      handleSetup();
    } else {
      handleUnlock();
    }
  };

  return (
    <div className="confirm-overlay">
      <div
        className="passphrase-dialog"
        role="dialog"
        aria-modal="true"
        onClick={e => e.stopPropagation()}
      >
        <form onSubmit={handleSubmit}>
          {mode === 'setup' ? (
            <>
              <h3>Set Up Encryption</h3>
              <p className="passphrase-description">
                Choose a passphrase to encrypt your credentials. Must be at least {MIN_PASSPHRASE_LEN} characters with uppercase, lowercase, digit, and special character.
              </p>

              <label className="passphrase-field">
                <span>Instance name</span>
                <input
                  type="text"
                  value={instance}
                  onChange={e => setInstance(e.target.value)}
                  placeholder="default"
                  disabled={loading}
                  onBlur={e => setInstance(e.target.value.trim())}
                  autoFocus
                />
              </label>

              <label className="passphrase-field">
                <span>Passphrase</span>
                <input
                  type="password"
                  value={passphrase}
                  onChange={e => setPassphrase(e.target.value)}
                  placeholder={`At least ${MIN_PASSPHRASE_LEN} characters`}
                  disabled={loading}
                />
              </label>

              <label className="passphrase-field">
                <span>Confirm passphrase</span>
                <input
                  type="password"
                  value={confirm}
                  onChange={e => setConfirm(e.target.value)}
                  placeholder="Re-enter passphrase"
                  disabled={loading}
                />
              </label>
            </>
          ) : (
            <>
              <h3>Unlock Encryption</h3>
              <p className="passphrase-description">
                Enter your passphrase to unlock encrypted credentials.
              </p>

              <label className="passphrase-field">
                <span>Passphrase</span>
                <input
                  type="password"
                  value={passphrase}
                  onChange={e => setPassphrase(e.target.value)}
                  placeholder="Enter your passphrase"
                  disabled={loading}
                  autoFocus
                />
              </label>
            </>
          )}

          <label className="passphrase-checkbox">
            <input
              type="checkbox"
              checked={saveToKeychain}
              onChange={e => setSaveToKeychain(e.target.checked)}
              disabled={loading}
            />
            <span>Save to system keychain</span>
          </label>

          {error && <p className="passphrase-error">{error}</p>}

          <div className="passphrase-actions">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={handleCancel}
              disabled={loading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={loading}
            >
              {loading ? 'Processing...' : mode === 'setup' ? 'Set Up' : 'Unlock'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
