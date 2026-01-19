import { useState, useCallback, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sidebar, ServerGroupCard, ServerGroupForm, ForwardingForm } from './components';
import { useServerGroups, useForwardings, useTunnelInstances, useBinaryPath } from './hooks';
import type { ServerGroup, Forwarding, ServerGroupFormData, ForwardingFormData, ImportResult } from './types';
import { serverGroupToForm, forwardingToForm } from './types';
import './App.css';

type View = 'list' | 'create-group' | 'edit-group' | 'create-forwarding' | 'edit-forwarding';

function App() {
  const { serverGroups, loading: groupsLoading, createServerGroup, updateServerGroup, deleteServerGroup, getServerGroup, refresh: refreshGroups } = useServerGroups();
  const { forwardings, loading: forwardingsLoading, createForwarding, updateForwarding, deleteForwarding, getForwardingsByGroup, getForwarding, refresh: refreshForwardings } = useForwardings();
  const { instances, startTunnel, stopTunnel, loading: instancesLoading } = useTunnelInstances();
  const { customBinaryPath, isUsingBundled, selectCustomBinaryPath, useBundledBinary } = useBinaryPath();

  // Hidden file input for import
  const importInputRef = useRef<HTMLInputElement>(null);

  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [selectedForwardingId, setSelectedForwardingId] = useState<string | null>(null);
  const [view, setView] = useState<View>('list');
  const [editingGroup, setEditingGroup] = useState<ServerGroup | null>(null);
  const [editingForwarding, setEditingForwarding] = useState<Forwarding | null>(null);
  const [addForwardingToGroupId, setAddForwardingToGroupId] = useState<string | null>(null);

  // Handlers for Server Groups
  const handleAddGroup = useCallback(() => {
    setView('create-group');
    setEditingGroup(null);
  }, []);

  const handleEditGroup = useCallback((group: ServerGroup) => {
    setEditingGroup(group);
    setView('edit-group');
  }, []);

  const handleCreateGroupSubmit = useCallback(async (form: ServerGroupFormData) => {
    try {
      await createServerGroup(form);
      setView('list');
    } catch (e) {
      alert(`Failed to create server group: ${e instanceof Error ? e.message : e}`);
    }
  }, [createServerGroup]);

  const handleEditGroupSubmit = useCallback(async (form: ServerGroupFormData) => {
    if (!editingGroup) {
      throw new Error('No editing group selected');
    }
    try {
      await updateServerGroup(editingGroup.id, form);
      setView('list');
      setEditingGroup(null);
    } catch (e) {
      alert(`Failed to update server group: ${e instanceof Error ? e.message : e}`);
    }
  }, [editingGroup, updateServerGroup]);

  const handleDeleteGroup = useCallback(async (id: string) => {
    if (window.confirm('Are you sure you want to delete this server group?')) {
      try {
        await deleteServerGroup(id);
        if (selectedGroupId === id) {
          setSelectedGroupId(null);
          setSelectedForwardingId(null);
        }
      } catch (e) {
        alert(`Failed to delete server group: ${e instanceof Error ? e.message : e}`);
      }
    }
  }, [deleteServerGroup, selectedGroupId]);

  // Handlers for Forwardings
  const handleAddForwarding = useCallback((groupId: string) => {
    setAddForwardingToGroupId(groupId);
    setEditingForwarding(null);
    setView('create-forwarding');
  }, []);

  const handleEditForwarding = useCallback((forwarding: Forwarding) => {
    setEditingForwarding(forwarding);
    setAddForwardingToGroupId(forwarding.server_group_id);
    setView('edit-forwarding');
  }, []);

  const handleCreateForwardingSubmit = useCallback(async (form: ForwardingFormData) => {
    if (!addForwardingToGroupId) {
      throw new Error('No server group selected');
    }
    try {
      await createForwarding(addForwardingToGroupId, form);
      setView('list');
      setAddForwardingToGroupId(null);
    } catch (e) {
      alert(`Failed to create forwarding: ${e instanceof Error ? e.message : e}`);
    }
  }, [addForwardingToGroupId, createForwarding]);

  const handleEditForwardingSubmit = useCallback(async (form: ForwardingFormData) => {
    if (!editingForwarding) {
      throw new Error('No editing forwarding selected');
    }
    try {
      await updateForwarding(editingForwarding.id, editingForwarding.server_group_id, form);
      setView('list');
      setEditingForwarding(null);
      setAddForwardingToGroupId(null);
    } catch (e) {
      alert(`Failed to update forwarding: ${e instanceof Error ? e.message : e}`);
    }
  }, [editingForwarding, updateForwarding]);

  const handleDeleteForwarding = useCallback(async (id: string) => {
    // Check if running
    const instance = instances.find(i => i.forwarding_id === id);
    if (instance && (instance.status === 'running' || instance.status === 'starting')) {
      alert('Cannot delete a running forwarding. Stop it first.');
      return;
    }
    if (window.confirm('Are you sure you want to delete this forwarding?')) {
      try {
        await deleteForwarding(id);
        if (selectedForwardingId === id) {
          setSelectedForwardingId(null);
        }
      } catch (e) {
        alert(`Failed to delete forwarding: ${e instanceof Error ? e.message : e}`);
      }
    }
  }, [deleteForwarding, selectedForwardingId, instances]);

  // Handlers for Tunnels
  const handleStartForwarding = useCallback(async (id: string) => {
    try {
      await startTunnel(id);
    } catch (e) {
      alert(`Failed to start tunnel: ${e instanceof Error ? e.message : e}`);
    }
  }, [startTunnel]);

  const handleStopForwarding = useCallback(async (id: string) => {
    try {
      await stopTunnel(id);
    } catch (e) {
      alert(`Failed to stop tunnel: ${e instanceof Error ? e.message : e}`);
    }
  }, [stopTunnel]);

  const handleCancel = useCallback(() => {
    setView('list');
    setEditingGroup(null);
    setEditingForwarding(null);
    setAddForwardingToGroupId(null);
  }, []);

  // Binary path handlers
  const handleSelectCustomBinaryPath = useCallback(async () => {
    try {
      await selectCustomBinaryPath();
    } catch (e) {
      alert(`Failed to set custom binary path: ${e instanceof Error ? e.message : e}`);
    }
  }, [selectCustomBinaryPath]);

  const handleUseBundledBinary = useCallback(async () => {
    try {
      await useBundledBinary();
    } catch (e) {
      alert(`Failed to switch to bundled binary: ${e instanceof Error ? e.message : e}`);
    }
  }, [useBundledBinary]);

  // Export/Import handlers
  const handleExport = useCallback(async () => {
    try {
      const json = await invoke<string>('export_configs');
      // Create a downloadable file
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'tunnel-rs-configs.json';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`Failed to export configs: ${e instanceof Error ? e.message : e}`);
    }
  }, []);

  const handleImportClick = useCallback(() => {
    importInputRef.current?.click();
  }, []);

  const handleImportFile = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const json = await file.text();
      const result = await invoke<ImportResult>('import_configs', { json });

      // Refresh data
      await refreshGroups();
      await refreshForwardings();

      // Show result
      if (result.success) {
        const messages: string[] = [];
        if (result.groups_imported > 0) {
          messages.push(`${result.groups_imported} server group(s) imported`);
        }
        if (result.forwardings_imported > 0) {
          messages.push(`${result.forwardings_imported} forwarding(s) imported`);
        }
        if (result.groups_skipped > 0) {
          messages.push(`${result.groups_skipped} server group(s) skipped`);
        }
        if (result.forwardings_skipped > 0) {
          messages.push(`${result.forwardings_skipped} forwarding(s) skipped`);
        }
        if (result.errors.length > 0) {
          messages.push(`Warnings: ${result.errors.join(', ')}`);
        }
        alert(`Import completed:\n${messages.join('\n')}`);
      } else {
        alert(`Import failed:\n${result.errors.join('\n')}`);
      }
    } catch (e) {
      alert(`Failed to import configs: ${e instanceof Error ? e.message : e}`);
    } finally {
      // Reset file input so the same file can be imported again
      e.target.value = '';
    }
  }, [refreshGroups, refreshForwardings]);

  // Sidebar selection handlers
  const handleSelectGroup = useCallback((id: string) => {
    setSelectedGroupId(id);
    setSelectedForwardingId(null);
  }, []);

  const handleSelectForwarding = useCallback((id: string) => {
    const forwarding = getForwarding(id);
    if (forwarding) {
      setSelectedGroupId(forwarding.server_group_id);
      setSelectedForwardingId(id);
    }
  }, [getForwarding]);

  // Memoized form data
  const editingGroupFormData = useMemo(
    () => editingGroup ? serverGroupToForm(editingGroup) : undefined,
    [editingGroup]
  );

  const editingForwardingFormData = useMemo(
    () => editingForwarding ? forwardingToForm(editingForwarding) : undefined,
    [editingForwarding]
  );

  const addForwardingGroupName = useMemo(() => {
    if (!addForwardingToGroupId) return '';
    const group = getServerGroup(addForwardingToGroupId);
    return group?.name ?? '';
  }, [addForwardingToGroupId, getServerGroup]);

  // Stats
  const runningCount = instances.filter(i => i.status === 'running' || i.status === 'starting').length;

  return (
    <div className="app">
      <Sidebar
        serverGroups={serverGroups}
        forwardings={forwardings}
        instances={instances}
        selectedGroupId={selectedGroupId}
        selectedForwardingId={selectedForwardingId}
        onSelectGroup={handleSelectGroup}
        onSelectForwarding={handleSelectForwarding}
        onAddGroup={handleAddGroup}
      />

      <main className="main-content">
        {view === 'create-group' && (
          <div className="form-container">
            <ServerGroupForm
              onSubmit={handleCreateGroupSubmit}
              onCancel={handleCancel}
            />
          </div>
        )}

        {view === 'edit-group' && editingGroupFormData && (
          <div className="form-container">
            <ServerGroupForm
              initial={editingGroupFormData}
              onSubmit={handleEditGroupSubmit}
              onCancel={handleCancel}
              isEditing
            />
          </div>
        )}

        {view === 'create-forwarding' && addForwardingToGroupId && (
          <div className="form-container">
            <ForwardingForm
              serverGroupName={addForwardingGroupName}
              onSubmit={handleCreateForwardingSubmit}
              onCancel={handleCancel}
            />
          </div>
        )}

        {view === 'edit-forwarding' && editingForwardingFormData && (
          <div className="form-container">
            <ForwardingForm
              initial={editingForwardingFormData}
              serverGroupName={addForwardingGroupName}
              onSubmit={handleEditForwardingSubmit}
              onCancel={handleCancel}
              isEditing
            />
          </div>
        )}

        {view === 'list' && (
          <>
            <header className="main-header">
              <h2>Server Groups</h2>
              <p className="header-subtitle">
                {serverGroups.length} group{serverGroups.length !== 1 ? 's' : ''} •
                {' '}{forwardings.length} forwarding{forwardings.length !== 1 ? 's' : ''} •
                {' '}{runningCount} running
              </p>
              <div className="binary-path-row">
                <span className="binary-path-info" title={isUsingBundled ? 'Bundled' : (customBinaryPath ?? 'Not set')}>
                  Binary: {isUsingBundled ? 'Bundled' : (customBinaryPath ?? 'Not set')}
                </span>
                <div className="binary-path-actions">
                  <button
                    className="btn-small"
                    onClick={handleSelectCustomBinaryPath}
                    title="Select custom binary path"
                  >
                    Use Custom
                  </button>
                  {!isUsingBundled && (
                    <button
                      className="btn-small btn-secondary"
                      onClick={handleUseBundledBinary}
                      title="Switch to bundled binary"
                    >
                      Use Bundled
                    </button>
                  )}
                </div>
              </div>
              <div className="export-import-row">
                <button
                  className="btn-small"
                  onClick={handleExport}
                  title="Export all configs to a shareable JSON file (without auth tokens)"
                >
                  Export
                </button>
                <button
                  className="btn-small"
                  onClick={handleImportClick}
                  title="Import configs from a JSON file"
                >
                  Import
                </button>
                <input
                  ref={importInputRef}
                  type="file"
                  accept=".json,application/json"
                  onChange={handleImportFile}
                  style={{ display: 'none' }}
                />
              </div>
            </header>

            {groupsLoading || forwardingsLoading ? (
              <div className="loading-state">
                <div className="spinner" />
                <p>Loading...</p>
              </div>
            ) : serverGroups.length === 0 ? (
              <div className="empty-main">
                <div className="empty-icon">🖥️</div>
                <h3>No server groups configured</h3>
                <p>Create your first server group to get started.</p>
                <button className="btn-primary" onClick={handleAddGroup}>
                  + Create Server Group
                </button>
              </div>
            ) : (
              <div className="cards-grid">
                {serverGroups.map(group => (
                  <ServerGroupCard
                    key={group.id}
                    group={group}
                    forwardings={getForwardingsByGroup(group.id)}
                    instances={instances}
                    onEdit={() => handleEditGroup(group)}
                    onDelete={() => handleDeleteGroup(group.id)}
                    onAddForwarding={() => handleAddForwarding(group.id)}
                    onEditForwarding={handleEditForwarding}
                    onDeleteForwarding={handleDeleteForwarding}
                    onStartForwarding={handleStartForwarding}
                    onStopForwarding={handleStopForwarding}
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
