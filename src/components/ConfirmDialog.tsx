// Confirmation dialog component

import { useEffect } from 'react';
import './ConfirmDialog.css';

interface ConfirmDialogProps {
    message: string;
    onConfirm: () => void;
    onCancel: () => void;
    loading?: boolean;
}

export function ConfirmDialog({ message, onConfirm, onCancel, loading = false }: ConfirmDialogProps) {
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape' && !loading) {
                onCancel();
            }
        };
        document.addEventListener('keydown', handleKeyDown);
        return () => document.removeEventListener('keydown', handleKeyDown);
    }, [onCancel, loading]);

    return (
        <div className="confirm-overlay" onClick={loading ? undefined : onCancel}>
            <div
                className="confirm-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="confirm-dialog-message"
                onClick={e => e.stopPropagation()}
            >
                <p id="confirm-dialog-message" className="confirm-message">{message}</p>
                <div className="confirm-actions">
                    <button className="btn btn-secondary" onClick={onCancel} disabled={loading}>
                        Cancel
                    </button>
                    <button className="btn btn-danger" onClick={onConfirm} disabled={loading}>
                        {loading ? 'Deleting...' : 'Delete'}
                    </button>
                </div>
            </div>
        </div>
    );
}
