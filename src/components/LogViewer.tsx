// Log viewer component for tunnel process output

import { useEffect, useRef } from 'react';
import type { LogEntry } from '../types';
import './LogViewer.css';

interface LogViewerProps {
    logs: LogEntry[];
    maxHeight?: string;
}

export function LogViewer({ logs, maxHeight = '300px' }: LogViewerProps) {
    const containerRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom when new logs arrive
    useEffect(() => {
        if (containerRef.current) {
            containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
    }, [logs]);

    const formatTime = (timestamp: number) => {
        const date = new Date(timestamp * 1000);
        return date.toLocaleTimeString();
    };

    if (logs.length === 0) {
        return (
            <div className="log-viewer empty" style={{ maxHeight }}>
                <span className="empty-text">No logs yet</span>
            </div>
        );
    }

    return (
        <div className="log-viewer" ref={containerRef} style={{ maxHeight }}>
            {logs.map((log, index) => (
                <div key={index} className={`log-entry ${log.is_error ? 'error' : ''}`}>
                    <span className="log-time">{formatTime(log.timestamp)}</span>
                    <span className="log-message">{log.message}</span>
                </div>
            ))}
        </div>
    );
}
