// Type definitions for tunnel-rs-manager

export interface IrohConfig {
  server_node_id: string;
  request_source?: string;
  target?: string;
  relay_urls?: string[];
  dns_server?: string;
  socks5_proxy?: string;
  auth_token?: string;
  auth_token_file?: string;
  transport?: {
    congestion_controller?: string;
    receive_window?: number;
    send_window?: number;
  };
}

export interface TunnelClientConfig {
  role: string;
  mode: string;
  iroh: IrohConfig;
}

export interface StoredConfig {
  id: string;
  name: string;
  config: TunnelClientConfig;
  created_at: number;
  updated_at: number;
}

export type TunnelStatus = 'stopped' | 'starting' | 'running' | 'error';

export interface LogEntry {
  timestamp: number;
  message: string;
  is_error: boolean;
}

export interface TunnelInstance {
  config_id: string;
  config_name: string;
  status: TunnelStatus;
  logs: LogEntry[];
}

// Form data for creating/editing configs
export interface ConfigFormData {
  name: string;
  server_node_id: string;
  source: string;
  target: string;
  auth_token: string;
  relay_urls: string;  // Comma-separated for form input
}

export const emptyConfigForm: ConfigFormData = {
  name: '',
  server_node_id: '',
  source: '',
  target: '',
  auth_token: '',
  relay_urls: '',
};

export function storedConfigToForm(config: StoredConfig): ConfigFormData {
  return {
    name: config.name,
    server_node_id: config.config.iroh.server_node_id,
    source: config.config.iroh.request_source || '',
    target: config.config.iroh.target || '',
    auth_token: config.config.iroh.auth_token || '',
    relay_urls: config.config.iroh.relay_urls?.join(', ') ?? '',
  };
}
