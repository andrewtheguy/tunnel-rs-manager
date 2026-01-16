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
    const prevLogsLengthRef = useRef(logs.length);
    const isNearBottomRef = useRef(true);

    // Track if user is near bottom
    const handleScroll = () => {
        if (containerRef.current) {
            const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
            // Consider "near bottom" if within 50px of the bottom
            isNearBottomRef.current = scrollHeight - scrollTop - clientHeight < 50;
        }
    };

    // Auto-scroll to bottom only when new logs arrive AND user is near bottom
    useEffect(() => {
        const hasNewLogs = logs.length > prevLogsLengthRef.current;
        prevLogsLengthRef.current = logs.length;

        if (hasNewLogs && isNearBottomRef.current && containerRef.current) {
            containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
    }, [logs.length]);

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
        <div className="log-viewer" ref={containerRef} style={{ maxHeight }} onScroll={handleScroll}>
            {logs.map((log, index) => (
                <div key={`${log.timestamp}-${index}`} className={`log-entry ${log.is_error ? 'error' : ''}`}>
                    <span className="log-time">{formatTime(log.timestamp)}</span>
                    <span className="log-message">{log.message}</span>
                </div>
            ))}
        </div>
    );
}
