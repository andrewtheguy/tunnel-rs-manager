// Forwarding item component showing individual forwarding details and controls

import { useState } from 'react';
import type { Forwarding, TunnelInstance } from '../types';
import { StatusBadge } from './StatusBadge';
import { LogViewer } from './LogViewer';
import './ForwardingItem.css';

interface ForwardingItemProps {
    forwarding: Forwarding;
    instance?: TunnelInstance;
    onStart: () => void;
    onStop: () => void;
    onEdit: () => void;
    onDelete: () => void;
    loading?: boolean;
}

export function ForwardingItem({
    forwarding,
    instance,
    onStart,
    onStop,
    onEdit,
    onDelete,
    loading = false,
}: ForwardingItemProps) {
    const [showLogs, setShowLogs] = useState(false);
    const status = instance?.status || 'stopped';
    const isRunning = status === 'running' || status === 'starting';
    const hasLogs = instance && instance.logs.length > 0;

    return (
        <div className={`forwarding-item ${isRunning ? 'active' : ''}`}>
            <div className="forwarding-main">
                <div className="forwarding-info">
                    <div className="forwarding-name-row">
                        <span className="forwarding-name">{forwarding.name}</span>
                        <StatusBadge status={status} size="small" />
                    </div>
                    <div className="forwarding-addresses">
                        <span className="address source" title={forwarding.source || 'No source'}>
                            {forwarding.source || <em>No source</em>}
                        </span>
                        <span className="arrow">→</span>
                        <span className="address target" title={forwarding.target || 'No target'}>
                            {forwarding.target || <em>No target</em>}
                        </span>
                    </div>
                </div>

                <div className="forwarding-actions">
                    {isRunning ? (
                        <button
                            className="btn-action btn-stop"
                            onClick={onStop}
                            disabled={loading}
                            title="Stop"
                        >
                            <StopIcon />
                        </button>
                    ) : (
                        <button
                            className="btn-action btn-start"
                            onClick={onStart}
                            disabled={loading}
                            title="Start"
                        >
                            <PlayIcon />
                        </button>
                    )}
                    <button
                        className="btn-icon"
                        onClick={onEdit}
                        disabled={isRunning || loading}
                        title={isRunning ? "Stop forwarding before editing" : "Edit"}
                    >
                        <EditIcon />
                    </button>
                    <button
                        className="btn-icon btn-danger"
                        onClick={onDelete}
                        disabled={isRunning || loading}
                        title={isRunning ? "Stop forwarding before deleting" : "Delete"}
                    >
                        <TrashIcon />
                    </button>
                    {hasLogs && (
                        <button
                            className={`btn-icon ${showLogs ? 'active' : ''}`}
                            onClick={() => setShowLogs(!showLogs)}
                            title={showLogs ? "Hide logs" : "Show logs"}
                        >
                            <LogIcon />
                        </button>
                    )}
                </div>
            </div>

            {showLogs && hasLogs && (
                <div className="forwarding-logs">
                    <LogViewer logs={instance!.logs.slice(-20)} maxHeight="150px" />
                </div>
            )}
        </div>
    );
}

// Icons
function PlayIcon() {
    return (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
        </svg>
    );
}

function StopIcon() {
    return (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
    );
}

function EditIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
    );
}

function TrashIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        </svg>
    );
}

function LogIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
        </svg>
    );
}
