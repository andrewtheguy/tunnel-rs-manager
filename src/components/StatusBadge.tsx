// Status badge component showing tunnel connection state

import type { TunnelStatus } from '../types';
import './StatusBadge.css';

interface StatusBadgeProps {
    status: TunnelStatus;
    size?: 'small' | 'normal';
}

export function StatusBadge({ status, size = 'normal' }: StatusBadgeProps) {
    return (
        <span className={`status-badge status-${status} size-${size}`}>
            <span className="status-dot" />
            <span className="status-text">{status}</span>
        </span>
    );
}
