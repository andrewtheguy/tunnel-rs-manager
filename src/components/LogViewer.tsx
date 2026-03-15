// Log viewer component for tunnel process output

import { useEffect, useRef, useCallback } from 'react';
import type { LogEntry } from '../types';
import './LogViewer.css';

interface LogViewerProps {
    logs: LogEntry[];
    maxHeight?: string;
    hasMore?: boolean;
    onLoadMore?: () => void;
}

export function LogViewer({ logs, maxHeight = '300px', hasMore = false, onLoadMore }: LogViewerProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const prevLogsLengthRef = useRef(logs.length);
    const isNearBottomRef = useRef(true);
    const isLoadingMoreRef = useRef(false);

    // Restore scroll position after prepending older logs
    const prevScrollHeightRef = useRef(0);

    // Track if user is near bottom
    const handleScroll = useCallback(() => {
        if (!containerRef.current) return;
        const { scrollTop, scrollHeight, clientHeight } = containerRef.current;

        // Consider "near bottom" if within 50px of the bottom
        isNearBottomRef.current = scrollHeight - scrollTop - clientHeight < 50;

        // Load more when scrolled to the top
        if (scrollTop === 0 && hasMore && onLoadMore && !isLoadingMoreRef.current) {
            isLoadingMoreRef.current = true;
            prevScrollHeightRef.current = scrollHeight;
            onLoadMore();
        }
    }, [hasMore, onLoadMore]);

    // After logs change from loading more, restore scroll position so it doesn't jump
    useEffect(() => {
        if (isLoadingMoreRef.current && containerRef.current) {
            const newScrollHeight = containerRef.current.scrollHeight;
            containerRef.current.scrollTop = newScrollHeight - prevScrollHeightRef.current;
            isLoadingMoreRef.current = false;
        }
    });

    // Auto-scroll to bottom only when new logs arrive AND user is near bottom
    useEffect(() => {
        const hasNewLogs = logs.length > prevLogsLengthRef.current;
        prevLogsLengthRef.current = logs.length;

        if (hasNewLogs && isNearBottomRef.current && containerRef.current && !isLoadingMoreRef.current) {
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
            {hasMore && (
                <div className="log-load-more">Scroll up to load more</div>
            )}
            {logs.map((log, index) => (
                <div key={`${log.timestamp}-${index}`} className={`log-entry ${log.is_error ? 'error' : ''}`}>
                    <span className="log-time">{formatTime(log.timestamp)}</span>
                    <span className="log-message">{log.message}</span>
                </div>
            ))}
        </div>
    );
}
