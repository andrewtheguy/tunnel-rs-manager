import { useState, useCallback, useMemo } from 'react';
import { Sidebar, TunnelCard, ConfigForm } from './components';
import { useTunnelConfigs, useTunnelInstances } from './hooks';
import type { StoredConfig, ConfigFormData } from './types';
import { storedConfigToForm } from './types';
import './App.css';

type View = 'list' | 'create' | 'edit';

function App() {
  const { configs, loading: configsLoading, createConfig, updateConfig, deleteConfig } = useTunnelConfigs();
  const { instances, startTunnel, stopTunnel, getInstance, loading: instancesLoading } = useTunnelInstances();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [view, setView] = useState<View>('list');
  const [editingConfig, setEditingConfig] = useState<StoredConfig | null>(null);

  const handleAdd = useCallback(() => {
    setView('create');
    setEditingConfig(null);
  }, []);

  const handleEdit = useCallback((config: StoredConfig) => {
    setEditingConfig(config);
    setView('edit');
  }, []);

  const handleCreateSubmit = useCallback(async (form: ConfigFormData) => {
    await createConfig(form);
    setView('list');
  }, [createConfig]);

  const handleEditSubmit = useCallback(async (form: ConfigFormData) => {
    if (editingConfig) {
      await updateConfig(editingConfig.id, form);
    }
    setView('list');
    setEditingConfig(null);
  }, [editingConfig, updateConfig]);

  const handleCancel = useCallback(() => {
    setView('list');
    setEditingConfig(null);
  }, []);

  const editingFormData = useMemo(
    () => editingConfig ? storedConfigToForm(editingConfig) : undefined,
    [editingConfig]
  );

  const handleDelete = useCallback(async (id: string) => {
    if (window.confirm('Are you sure you want to delete this configuration?')) {
      await deleteConfig(id);
      if (selectedId === id) {
        setSelectedId(null);
      }
    }
  }, [deleteConfig, selectedId]);

  const handleStart = useCallback(async (id: string) => {
    try {
      await startTunnel(id);
    } catch (e) {
      alert(`Failed to start tunnel: ${e instanceof Error ? e.message : e}`);
    }
  }, [startTunnel]);

  const handleStop = useCallback(async (id: string) => {
    try {
      await stopTunnel(id);
    } catch (e) {
      alert(`Failed to stop tunnel: ${e instanceof Error ? e.message : e}`);
    }
  }, [stopTunnel]);

  return (
    <div className="app">
      <Sidebar
        configs={configs}
        instances={instances}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onAdd={handleAdd}
      />

      <main className="main-content">
        {view === 'create' && (
          <div className="form-container">
            <ConfigForm
              onSubmit={handleCreateSubmit}
              onCancel={handleCancel}
            />
          </div>
        )}

        {view === 'edit' && editingFormData && (
          <div className="form-container">
            <ConfigForm
              initial={editingFormData}
              onSubmit={handleEditSubmit}
              onCancel={handleCancel}
              isEditing
            />
          </div>
        )}

        {view === 'list' && (
          <>
            <header className="main-header">
              <h2>Tunnel Configurations</h2>
              <p className="header-subtitle">
                {configs.length} configuration{configs.length !== 1 ? 's' : ''} •
                {' '}{instances.filter(i => i.status === 'running').length} running
              </p>
            </header>

            {configsLoading ? (
              <div className="loading-state">
                <div className="spinner" />
                <p>Loading configurations...</p>
              </div>
            ) : configs.length === 0 ? (
              <div className="empty-main">
                <div className="empty-icon">🚇</div>
                <h3>No tunnels configured</h3>
                <p>Create your first tunnel configuration to get started.</p>
                <button className="btn-primary" onClick={handleAdd}>
                  + Create Configuration
                </button>
              </div>
            ) : (
              <div className="cards-grid">
                {configs.map(config => (
                  <TunnelCard
                    key={config.id}
                    config={config}
                    instance={getInstance(config.id)}
                    onStart={() => handleStart(config.id)}
                    onStop={() => handleStop(config.id)}
                    onEdit={() => handleEdit(config)}
                    onDelete={() => handleDelete(config.id)}
                    loading={instancesLoading}
                  />
                ))}
              </div>
            )}
          </>
        )}
      </main>
    </div>
  );
}

export default App;
