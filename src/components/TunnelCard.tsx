// Tunnel card component showing config details and controls

import type { StoredConfig, TunnelInstance } from '../types';
import { StatusBadge } from './StatusBadge';
import { LogViewer } from './LogViewer';
import './TunnelCard.css';

interface TunnelCardProps {
    config: StoredConfig;
    instance?: TunnelInstance;
    onStart: () => void;
    onStop: () => void;
    onEdit: () => void;
    onDelete: () => void;
    loading?: boolean;
}

export function TunnelCard({
    config,
    instance,
    onStart,
    onStop,
    onEdit,
    onDelete,
    loading = false,
}: TunnelCardProps) {
    const status = instance?.status || 'stopped';
    const isRunning = status === 'running' || status === 'starting';

    return (
        <div className={`tunnel-card ${isRunning ? 'active' : ''}`}>
            <div className="card-header">
                <div className="card-title-row">
                    <h3 className="card-title">{config.name}</h3>
                    <StatusBadge status={status} />
                </div>
                <p className="card-subtitle">{config.config.iroh.server_node_id.slice(0, 16)}...</p>
            </div>

            <div className="card-body">
                <div className="config-details">
                    <div className="detail-row">
                        <span className="detail-label">Source</span>
                        <span className="detail-value">
                            {config.config.iroh.request_source || <em className="empty">Not set</em>}
                        </span>
                    </div>
                    <div className="detail-row">
                        <span className="detail-label">Target</span>
                        <span className="detail-value">
                            {config.config.iroh.target || <em className="empty">Not set</em>}
                        </span>
                    </div>
                    {config.config.iroh.relay_urls.length > 0 && (
                        <div className="detail-row">
                            <span className="detail-label">Relays</span>
                            <span className="detail-value">{config.config.iroh.relay_urls.length} configured</span>
                        </div>
                    )}
                </div>

                {instance && instance.logs.length > 0 && (
                    <div className="card-logs">
                        <LogViewer logs={instance.logs.slice(-10)} maxHeight="150px" />
                    </div>
                )}
            </div>

            <div className="card-actions">
                <div className="action-group left">
                    {isRunning ? (
                        <button
                            className="btn-action btn-stop"
                            onClick={onStop}
                            disabled={loading}
                            title="Stop tunnel"
                        >
                            <StopIcon />
                            Stop
                        </button>
                    ) : (
                        <button
                            className="btn-action btn-start"
                            onClick={onStart}
                            disabled={loading}
                            title="Start tunnel"
                        >
                            <PlayIcon />
                            Start
                        </button>
                    )}
                </div>
                <div className="action-group right">
                    <button
                        className="btn-icon"
                        onClick={onEdit}
                        disabled={isRunning}
                        title="Edit configuration"
                    >
                        <EditIcon />
                    </button>
                    <button
                        className="btn-icon btn-danger"
                        onClick={onDelete}
                        disabled={isRunning}
                        title="Delete configuration"
                    >
                        <TrashIcon />
                    </button>
                </div>
            </div>
        </div>
    );
}

// Icons
function PlayIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
        </svg>
    );
}

function StopIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
    );
}

function EditIcon() {
    return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
    );
}

function TrashIcon() {
    return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        </svg>
    );
}
