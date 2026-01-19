// Type definitions for tunnel-rs-manager

// ============================================================================
// Server Group Types
// ============================================================================

/** Server Group: Named collection of shared connection settings */
export interface ServerGroup {
  id: string;
  name: string;
  server_node_id: string;
  auth_token?: string;
  relay_urls: string[];
  created_at: number;
  updated_at: number;
}

/** Form data for creating/editing server groups */
export interface ServerGroupFormData {
  name: string;
  server_node_id: string;
  auth_token: string;
  relay_urls: string; // Comma-separated for form input
}

export const emptyServerGroupForm: ServerGroupFormData = {
  name: '',
  server_node_id: '',
  auth_token: '',
  relay_urls: '',
};

export function serverGroupToForm(group: ServerGroup): ServerGroupFormData {
  return {
    name: group.name,
    server_node_id: group.server_node_id,
    auth_token: group.auth_token ?? '',
    relay_urls: (group.relay_urls ?? []).join(', '),
  };
}

// ============================================================================
// Forwarding Types
// ============================================================================

/** Forwarding: Individual named source/target pair within a server group */
export interface Forwarding {
  id: string;
  server_group_id: string;
  name: string;
  source?: string;
  target?: string;
  created_at: number;
  updated_at: number;
}

/** Form data for creating/editing forwardings */
export interface ForwardingFormData {
  name: string;
  source: string;
  target: string;
}

export const emptyForwardingForm: ForwardingFormData = {
  name: '',
  source: '',
  target: '',
};

export function forwardingToForm(forwarding: Forwarding): ForwardingFormData {
  return {
    name: forwarding.name,
    source: forwarding.source ?? '',
    target: forwarding.target ?? '',
  };
}

// ============================================================================
// Tunnel Instance Types
// ============================================================================

export type TunnelStatus = 'stopped' | 'starting' | 'running' | 'error';

export interface LogEntry {
  timestamp: number;
  message: string;
  is_error: boolean;
}

export interface TunnelInstance {
  forwarding_id: string;
  forwarding_name: string;
  server_group_name: string;
  status: TunnelStatus;
  logs: LogEntry[];
}

// ============================================================================
// Export/Import Types
// ============================================================================

/** Server group for export (without auth_token) */
export interface ExportServerGroup {
  id: string;
  name: string;
  server_node_id: string;
  relay_urls?: string[];
}

/** Forwarding for export */
export interface ExportForwarding {
  id: string;
  server_group_id: string;
  name: string;
  source?: string;
  target?: string;
}

/** Export data format (shareable, no secrets) */
export interface ExportData {
  version: number;
  exported_at: number;
  server_groups: ExportServerGroup[];
  forwardings: ExportForwarding[];
}

/** Result of import operation */
export interface ImportResult {
  success: boolean;
  groups_imported: number;
  forwardings_imported: number;
  groups_skipped: number;
  forwardings_skipped: number;
  errors: string[];
}
