// Sidebar component showing config list

import type { StoredConfig, TunnelInstance } from '../types';
import { StatusBadge } from './StatusBadge';
import './Sidebar.css';

interface SidebarProps {
    configs: StoredConfig[];
    instances: TunnelInstance[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onAdd: () => void;
}

export function Sidebar({ configs, instances, selectedId, onSelect, onAdd }: SidebarProps) {
    const getInstanceStatus = (configId: string) => {
        const instance = instances.find(i => i.config_id === configId);
        return instance?.status || 'stopped';
    };

    return (
        <aside className="sidebar">
            <div className="sidebar-header">
                <h1 className="sidebar-title">
                    <TunnelIcon />
                    tunnel-rs
                </h1>
                <button className="btn-add" onClick={onAdd} title="Add tunnel configuration">
                    <PlusIcon />
                </button>
            </div>

            <div className="sidebar-content">
                {configs.length === 0 ? (
                    <div className="empty-state">
                        <p>No configurations yet</p>
                        <button className="btn-empty-add" onClick={onAdd}>
                            <PlusIcon />
                            Add your first tunnel
                        </button>
                    </div>
                ) : (
                    <ul className="config-list">
                        {configs.map(config => {
                            const status = getInstanceStatus(config.id);
                            return (
                                <li
                                    key={config.id}
                                    className={`config-item ${selectedId === config.id ? 'selected' : ''} ${status !== 'stopped' ? 'active' : ''}`}
                                    onClick={() => onSelect(config.id)}
                                >
                                    <div className="config-item-content">
                                        <span className="config-name">{config.name}</span>
                                        <span className="config-target">
                                            {config.config.iroh.target || 'No target set'}
                                        </span>
                                    </div>
                                    <StatusBadge status={status} size="small" />
                                </li>
                            );
                        })}
                    </ul>
                )}
            </div>

            <div className="sidebar-footer">
                <span className="version">v0.1.0</span>
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
