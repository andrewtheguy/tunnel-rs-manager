// Server Group card component showing group details and its forwardings

import { useMemo } from 'react';
import type { ServerGroup, Forwarding, TunnelInstance } from '../types';
import { ForwardingItem } from './ForwardingItem';
import './ServerGroupCard.css';

interface ServerGroupCardProps {
    group: ServerGroup;
    forwardings: Forwarding[];
    instances: TunnelInstance[];
    onEdit: () => void;
    onDelete: () => void;
    onAddForwarding: () => void;
    onEditForwarding: (forwarding: Forwarding) => void;
    onDeleteForwarding: (id: string) => void;
    onStartForwarding: (id: string) => void;
    onStopForwarding: (id: string) => void;
    onExportForwardingToml: (id: string) => void;
    loading?: boolean;
}

export function ServerGroupCard({
    group,
    forwardings,
    instances,
    onEdit,
    onDelete,
    onAddForwarding,
    onEditForwarding,
    onDeleteForwarding,
    onStartForwarding,
    onStopForwarding,
    onExportForwardingToml,
    loading = false,
}: ServerGroupCardProps) {
    // Precompute instance lookup map for O(1) access
    const instanceByForwardingId = useMemo(() => new Map(
        instances.map(i => [i.forwarding_id, i])
    ), [instances]);

    // Count running forwardings and derive boolean from it
    const runningCount = forwardings.filter(f => {
        const instance = instanceByForwardingId.get(f.id);
        return instance && (instance.status === 'running' || instance.status === 'starting' || instance.status === 'reconnecting');
    }).length;
    const hasRunningForwarding = runningCount > 0;

    return (
        <div id={`group-card-${group.id}`} className={`server-group-card ${hasRunningForwarding ? 'active' : ''}`}>
            <div className="group-header">
                <div className="group-info">
                    <h3 className="group-name">{group.name}</h3>
                    <p className="group-node-id" title={group.server_node_id}>
                        {group.server_node_id.slice(0, 16)}...
                    </p>
                    <div className="group-meta">
                        <span className="meta-item">
                            {forwardings.length} forwarding{forwardings.length !== 1 ? 's' : ''}
                        </span>
                        {runningCount > 0 && (
                            <span className="meta-item running">
                                {runningCount} running
                            </span>
                        )}
                        {(group.relay_urls?.length ?? 0) > 0 && (
                            <span className="meta-item">
                                {group.relay_urls.length} relay{group.relay_urls.length !== 1 ? 's' : ''}
                            </span>
                        )}
                    </div>
                </div>
                <div className="group-actions">
                    <button
                        className="btn-icon"
                        onClick={onEdit}
                        disabled={hasRunningForwarding || loading}
                        title={hasRunningForwarding ? "Stop all forwardings before editing" : "Edit server group"}
                    >
                        <EditIcon />
                    </button>
                    <button
                        className="btn-icon btn-danger"
                        onClick={onDelete}
                        disabled={hasRunningForwarding || forwardings.length > 0 || loading}
                        title={
                            hasRunningForwarding
                                ? "Stop all forwardings before deleting"
                                : forwardings.length > 0
                                    ? "Delete all forwardings first"
                                    : "Delete server group"
                        }
                    >
                        <TrashIcon />
                    </button>
                </div>
            </div>

            <div className="forwardings-section">
                <div className="forwardings-header">
                    <span className="forwardings-title">Forwardings</span>
                    <button
                        className="btn-add-forwarding"
                        onClick={onAddForwarding}
                        title="Add forwarding"
                    >
                        <PlusIcon />
                        Add
                    </button>
                </div>

                {forwardings.length === 0 ? (
                    <div className="no-forwardings">
                        No forwardings configured
                    </div>
                ) : (
                    <div className="forwardings-list">
                        {forwardings.map(forwarding => (
                            <ForwardingItem
                                key={forwarding.id}
                                forwarding={forwarding}
                                instance={instanceByForwardingId.get(forwarding.id)}
                                onStart={() => onStartForwarding(forwarding.id)}
                                onStop={() => onStopForwarding(forwarding.id)}
                                onEdit={() => onEditForwarding(forwarding)}
                                onDelete={() => onDeleteForwarding(forwarding.id)}
                                onExportToml={() => onExportForwardingToml(forwarding.id)}
                                loading={loading}
                            />
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}

// Icons
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

function PlusIcon() {
    return (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
    );
}
