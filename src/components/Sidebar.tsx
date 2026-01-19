// Sidebar component showing hierarchical server groups and forwardings

import { useState, useMemo, useCallback } from 'react';
import type { ServerGroup, Forwarding, TunnelInstance } from '../types';
import { StatusBadge } from './StatusBadge';
import { version } from '../../package.json';
import './Sidebar.css';

interface SidebarProps {
    serverGroups: ServerGroup[];
    forwardings: Forwarding[];
    instances: TunnelInstance[];
    selectedGroupId: string | null;
    selectedForwardingId: string | null;
    onSelectGroup: (id: string) => void;
    onSelectForwarding: (id: string) => void;
    onAddGroup: () => void;
}

export function Sidebar({
    serverGroups,
    forwardings,
    instances,
    selectedGroupId,
    selectedForwardingId,
    onSelectGroup,
    onSelectForwarding,
    onAddGroup,
}: SidebarProps) {
    const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());

    const instancesByForwardingId = useMemo(() => new Map(
        instances.map(i => [i.forwarding_id, i])
    ), [instances]);

    const toggleGroup = (groupId: string) => {
        setCollapsedGroups(prev => {
            const next = new Set(prev);
            if (next.has(groupId)) {
                next.delete(groupId);
            } else {
                next.add(groupId);
            }
            return next;
        });
    };

    // Memoize forwardings grouped by server_group_id to avoid repeated filtering
    const forwardingsByGroup = useMemo(() => {
        const map = new Map<string, Forwarding[]>();
        for (const f of forwardings) {
            const list = map.get(f.server_group_id) || [];
            list.push(f);
            map.set(f.server_group_id, list);
        }
        return map;
    }, [forwardings]);

    const getForwardingsForGroup = useCallback((groupId: string) => {
        return forwardingsByGroup.get(groupId) || [];
    }, [forwardingsByGroup]);

    const getInstanceStatus = useCallback((forwardingId: string) => {
        const instance = instancesByForwardingId.get(forwardingId);
        return instance?.status || 'stopped';
    }, [instancesByForwardingId]);

    const getGroupRunningCount = useCallback((groupId: string) => {
        const groupForwardings = getForwardingsForGroup(groupId);
        return groupForwardings.filter(f => {
            const status = getInstanceStatus(f.id);
            return status === 'running' || status === 'starting';
        }).length;
    }, [getForwardingsForGroup, getInstanceStatus]);

    const totalRunning = instances.filter(
        i => i.status === 'running' || i.status === 'starting'
    ).length;

    return (
        <aside className="sidebar">
            <div className="sidebar-header">
                <h1 className="sidebar-title">
                    <TunnelIcon />
                    tunnel-rs
                </h1>
                <button className="btn-add" onClick={onAddGroup} title="Add server group">
                    <PlusIcon />
                </button>
            </div>

            <div className="sidebar-stats">
                <span>{serverGroups.length} group{serverGroups.length !== 1 ? 's' : ''}</span>
                <span className="separator">•</span>
                <span>{forwardings.length} forwarding{forwardings.length !== 1 ? 's' : ''}</span>
                {totalRunning > 0 && (
                    <>
                        <span className="separator">•</span>
                        <span className="running-count">{totalRunning} running</span>
                    </>
                )}
            </div>

            <div className="sidebar-content">
                {serverGroups.length === 0 ? (
                    <div className="empty-state">
                        <p>No server groups yet</p>
                        <button className="btn-empty-add" onClick={onAddGroup}>
                            <PlusIcon />
                            Add your first server group
                        </button>
                    </div>
                ) : (
                    <ul className="group-list">
                        {serverGroups.map(group => {
                            const groupForwardings = getForwardingsForGroup(group.id);
                            const runningCount = getGroupRunningCount(group.id);
                            const isCollapsed = collapsedGroups.has(group.id);
                            const isGroupSelected = selectedGroupId === group.id && !selectedForwardingId;

                            return (
                                <li key={group.id} className="group-item">
                                    <div
                                        className={`group-header-row ${isGroupSelected ? 'selected' : ''} ${runningCount > 0 ? 'active' : ''}`}
                                        onClick={() => onSelectGroup(group.id)}
                                    >
                                        <button
                                            className="collapse-btn"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                toggleGroup(group.id);
                                            }}
                                            title={isCollapsed ? 'Expand' : 'Collapse'}
                                        >
                                            <ChevronIcon collapsed={isCollapsed} />
                                        </button>
                                        <div className="group-header-content">
                                            <span className="group-name">{group.name}</span>
                                            <span className="group-info">
                                                {groupForwardings.length} fwd{groupForwardings.length !== 1 ? 's' : ''}
                                                {runningCount > 0 && (
                                                    <span className="running-indicator"> • {runningCount} running</span>
                                                )}
                                            </span>
                                        </div>
                                    </div>

                                    {!isCollapsed && groupForwardings.length > 0 && (
                                        <ul className="forwarding-list">
                                            {groupForwardings.map(forwarding => {
                                                const status = getInstanceStatus(forwarding.id);
                                                const isActive = status === 'running' || status === 'starting';
                                                const isSelected = selectedForwardingId === forwarding.id;

                                                return (
                                                    <li
                                                        key={forwarding.id}
                                                        className={`forwarding-item ${isSelected ? 'selected' : ''} ${isActive ? 'active' : ''}`}
                                                        onClick={() => onSelectForwarding(forwarding.id)}
                                                    >
                                                        <div className="forwarding-content">
                                                            <span className="forwarding-name">{forwarding.name}</span>
                                                            <span className="forwarding-target">
                                                                {forwarding.target || 'No target'}
                                                            </span>
                                                        </div>
                                                        <StatusBadge status={status} size="small" />
                                                    </li>
                                                );
                                            })}
                                        </ul>
                                    )}
                                </li>
                            );
                        })}
                    </ul>
                )}
            </div>

            <div className="sidebar-footer">
                <span className="version">v{version}</span>
            </div>
        </aside>
    );
}

// Icons
function TunnelIcon() {
    return (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z" />
            <path d="M12 6v12" />
            <path d="M6 12h12" />
        </svg>
    );
}

function PlusIcon() {
    return (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
    );
}

function ChevronIcon({ collapsed }: { collapsed: boolean }) {
    return (
        <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            style={{
                transform: collapsed ? 'rotate(-90deg)' : 'rotate(0deg)',
                transition: 'transform 0.15s ease',
            }}
        >
            <polyline points="6 9 12 15 18 9" />
        </svg>
    );
}
