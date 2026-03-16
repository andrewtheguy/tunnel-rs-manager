import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { Sidebar, ServerGroupCard, ServerGroupForm, ForwardingForm, ConfirmDialog, AgeKeyDialog } from './components';
import { useServerGroups, useForwardings, useTunnelInstances, useBinaryPath } from './hooks';
import type { ServerGroup, Forwarding, ServerGroupFormData, ForwardingFormData, ImportResult } from './types';
import { serverGroupToForm, forwardingToForm } from './types';
import './App.css';

type View = 'list' | 'create-group' | 'edit-group' | 'create-forwarding' | 'edit-forwarding';

type PendingAction =
  | { type: 'export' }
  | { type: 'import'; json: string }
  | { type: 'toml-export'; forwardingId: string };

function App() {
  const { serverGroups, loading: groupsLoading, createServerGroup, updateServerGroup, deleteServerGroup, getServerGroup, refresh: refreshGroups } = useServerGroups();
  const { forwardings, loading: forwardingsLoading, createForwarding, updateForwarding, deleteForwarding, getForwardingsByGroup, getForwarding, refresh: refreshForwardings } = useForwardings();
  const { instances, startTunnel, stopTunnel, loading: instancesLoading } = useTunnelInstances();
  const { customBinaryPath, isUsingBundled, binaryVersion, selectCustomBinaryPath, useBundledBinary } = useBinaryPath();

  // Hidden file input for import
  const importInputRef = useRef<HTMLInputElement>(null);
  // Ref to main content for scroll restoration
  const mainContentRef = useRef<HTMLElement>(null);
  // Saved scroll position when leaving list view
  const savedScrollPos = useRef(0);

  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [selectedForwardingId, setSelectedForwardingId] = useState<string | null>(null);
  const [view, setView] = useState<View>('list');
  const [editingGroup, setEditingGroup] = useState<ServerGroup | null>(null);
  const [editingForwarding, setEditingForwarding] = useState<Forwarding | null>(null);
  const [addForwardingToGroupId, setAddForwardingToGroupId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<{ type: 'group' | 'forwarding'; id: string; name: string } | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Age key dialog state
  const [showAgeKeyDialog, setShowAgeKeyDialog] = useState(false);
  const [ageKeyMode, setAgeKeyMode] = useState<'setup' | 'select'>('setup');
  const [ageKeyRecipients, setAgeKeyRecipients] = useState<string[]>([]);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);

  // Restore scroll position when returning to list view
  useEffect(() => {
    if (view === 'list' && mainContentRef.current && savedScrollPos.current > 0) {
      const rafId = requestAnimationFrame(() => {
        if (mainContentRef.current) {
          mainContentRef.current.scrollTop = savedScrollPos.current;
        }
      });
      return () => cancelAnimationFrame(rafId);
    }
  }, [view]);

  // Save scroll position before leaving list view
  const saveScrollPosition = useCallback(() => {
    if (mainContentRef.current) {
      savedScrollPos.current = mainContentRef.current.scrollTop;
    }
  }, []);

  // Handlers for Server Groups
  const handleAddGroup = useCallback(() => {
    saveScrollPosition();
    setView('create-group');
    setEditingGroup(null);
  }, [saveScrollPosition]);

  const handleEditGroup = useCallback((group: ServerGroup) => {
    saveScrollPosition();
    setEditingGroup(group);
    setView('edit-group');
  }, [saveScrollPosition]);

  const handleCreateGroupSubmit = useCallback(async (form: ServerGroupFormData) => {
    try {
      await createServerGroup(form);
      setView('list');
    } catch (e) {
      alert(`Failed to create server group: ${e instanceof Error ? e.message : e}`);
    }
  }, [createServerGroup]);

  const handleEditGroupSubmit = useCallback(async (form: ServerGroupFormData) => {
    try {
      if (!editingGroup) {
        throw new Error('No editing group selected');
      }
      await updateServerGroup(editingGroup.id, form);
      setView('list');
      setEditingGroup(null);
    } catch (e) {
      alert(`Failed to update server group: ${e instanceof Error ? e.message : e}`);
    }
  }, [editingGroup, updateServerGroup]);

  const handleDeleteGroup = useCallback((id: string) => {
    const group = getServerGroup(id);
    setPendingDelete({ type: 'group', id, name: group?.name || 'Server Group' });
  }, [getServerGroup]);

  const confirmDeleteGroup = useCallback(async (id: string) => {
    try {
      await deleteServerGroup(id);
      if (selectedGroupId === id) {
        setSelectedGroupId(null);
        setSelectedForwardingId(null);
      }
    } catch (e) {
      alert(`Failed to delete server group: ${e instanceof Error ? e.message : e}`);
    }
  }, [deleteServerGroup, selectedGroupId]);

  // Handlers for Forwardings
  const handleAddForwarding = useCallback((groupId: string) => {
    saveScrollPosition();
    setAddForwardingToGroupId(groupId);
    setEditingForwarding(null);
    setView('create-forwarding');
  }, [saveScrollPosition]);

  const handleEditForwarding = useCallback((forwarding: Forwarding) => {
    saveScrollPosition();
    setEditingForwarding(forwarding);
    setAddForwardingToGroupId(forwarding.server_group_id);
    setView('edit-forwarding');
  }, [saveScrollPosition]);

  const handleCreateForwardingSubmit = useCallback(async (form: ForwardingFormData) => {
    try {
      if (!addForwardingToGroupId) {
        throw new Error('No server group selected');
      }
      await createForwarding(addForwardingToGroupId, form);
      setView('list');
      setAddForwardingToGroupId(null);
    } catch (e) {
      alert(`Failed to create forwarding: ${e instanceof Error ? e.message : e}`);
    }
  }, [addForwardingToGroupId, createForwarding]);

  const handleEditForwardingSubmit = useCallback(async (form: ForwardingFormData) => {
    try {
      if (!editingForwarding) {
        throw new Error('No editing forwarding selected');
      }
      await updateForwarding(editingForwarding.id, editingForwarding.server_group_id, form);
      setView('list');
      setEditingForwarding(null);
      setAddForwardingToGroupId(null);
    } catch (e) {
      alert(`Failed to update forwarding: ${e instanceof Error ? e.message : e}`);
    }
  }, [editingForwarding, updateForwarding]);

  const handleDeleteForwarding = useCallback((id: string) => {
    const instance = instances.find(i => i.forwarding_id === id);
    if (instance && (instance.status === 'running' || instance.status === 'starting' || instance.status === 'reconnecting')) {
      alert('Cannot delete a running forwarding. Stop it first.');
      return;
    }
    const forwarding = getForwarding(id);
    setPendingDelete({ type: 'forwarding', id, name: forwarding?.name || 'Forwarding' });
  }, [instances, getForwarding]);

  const confirmDeleteForwarding = useCallback(async (id: string) => {
    try {
      await deleteForwarding(id);
      if (selectedForwardingId === id) {
        setSelectedForwardingId(null);
      }
    } catch (e) {
      alert(`Failed to delete forwarding: ${e instanceof Error ? e.message : e}`);
    }
  }, [deleteForwarding, selectedForwardingId]);

  const handleConfirmDelete = useCallback(async () => {
    if (!pendingDelete || deleting) return;
    setDeleting(true);
    try {
      if (pendingDelete.type === 'group') {
        await confirmDeleteGroup(pendingDelete.id);
      } else {
        await confirmDeleteForwarding(pendingDelete.id);
      }
    } finally {
      setDeleting(false);
      setPendingDelete(null);
    }
  }, [pendingDelete, deleting, confirmDeleteGroup, confirmDeleteForwarding]);

  const handleCancelDelete = useCallback(() => {
    setPendingDelete(null);
  }, []);

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

  // Age key resolution: returns recipient or opens dialog
  const resolveRecipient = useCallback(async (action: PendingAction): Promise<string | null> => {
    const keyExists = await invoke<boolean>('check_age_key_exists');
    if (!keyExists) {
      setPendingAction(action);
      setAgeKeyMode('setup');
      setAgeKeyRecipients([]);
      setShowAgeKeyDialog(true);
      return null;
    }
    const recipients = await invoke<string[]>('list_age_recipients');
    if (recipients.length === 0) {
      // Key file exists but no valid keys — treat as setup
      setPendingAction(action);
      setAgeKeyMode('setup');
      setAgeKeyRecipients([]);
      setShowAgeKeyDialog(true);
      return null;
    }
    if (recipients.length === 1) {
      return recipients[0];
    }
    // Multiple keys — show select dialog
    setPendingAction(action);
    setAgeKeyMode('select');
    setAgeKeyRecipients(recipients);
    setShowAgeKeyDialog(true);
    return null;
  }, []);

  // Perform the pending action with the resolved recipient
  const performActionWithRecipient = useCallback(async (recipient: string, action: PendingAction) => {
    try {
      if (action.type === 'export') {
        const json = await invoke<string>('export_configs', { recipient });
        const filePath = await save({
          defaultPath: 'tunnel-rs-configs.json',
          filters: [{ name: 'JSON', extensions: ['json'] }],
        });
        if (filePath) {
          await writeTextFile(filePath, json);
        }
      } else if (action.type === 'import') {
        const result = await invoke<ImportResult>('import_configs', { json: action.json });
        await Promise.all([refreshGroups(), refreshForwardings()]);
        const messages: string[] = [];
        if (result.groups_imported > 0) messages.push(`${result.groups_imported} server group(s) imported`);
        if (result.forwardings_imported > 0) messages.push(`${result.forwardings_imported} forwarding(s) imported`);
        if (result.groups_skipped > 0) messages.push(`${result.groups_skipped} server group(s) skipped`);
        if (result.forwardings_skipped > 0) messages.push(`${result.forwardings_skipped} forwarding(s) skipped`);
        if (result.errors.length > 0) messages.push(`Warnings: ${result.errors.join(', ')}`);
        if (result.success) {
          alert(`Import completed:\n${messages.join('\n')}`);
        } else {
          alert(`Import failed:\n${result.errors.join('\n')}`);
        }
      } else if (action.type === 'toml-export') {
        const tomlContent = await invoke<string>('export_forwarding_toml', { forwardingId: action.forwardingId, recipient });
        const forwarding = getForwarding(action.forwardingId);
        const defaultName = forwarding ? `${forwarding.name}.toml` : 'forwarding.toml';
        const filePath = await save({
          defaultPath: defaultName,
          filters: [{ name: 'TOML', extensions: ['toml'] }],
        });
        if (filePath) {
          await writeTextFile(filePath, tomlContent);
        }
      }
    } catch (e) {
      alert(`Operation failed: ${e instanceof Error ? e.message : e}`);
    }
  }, [refreshGroups, refreshForwardings, getForwarding]);

  // Export handler
  const handleExport = useCallback(async () => {
    try {
      const recipient = await resolveRecipient({ type: 'export' });
      if (recipient) {
        await performActionWithRecipient(recipient, { type: 'export' });
      }
    } catch (e) {
      alert(`Export failed: ${e instanceof Error ? e.message : e}`);
    }
  }, [resolveRecipient, performActionWithRecipient]);

  // Import handler
  const handleImportClick = useCallback(() => {
    importInputRef.current?.click();
  }, []);

  const handleImportFile = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    try {
      const json = await file.text();
      const hasCreds = await invoke<boolean>('check_import_has_credentials', { json });

      if (hasCreds) {
        // Need age key to decrypt — check key exists
        const keyExists = await invoke<boolean>('check_age_key_exists');
        if (!keyExists) {
          setPendingAction({ type: 'import', json });
          setAgeKeyMode('setup');
          setAgeKeyRecipients([]);
          setShowAgeKeyDialog(true);
        } else {
          // Key exists, proceed directly — import doesn't need recipient selection
          await performActionWithRecipient('', { type: 'import', json });
        }
      } else {
        // No credentials, import directly
        const result = await invoke<ImportResult>('import_configs', { json });
        await Promise.all([refreshGroups(), refreshForwardings()]);
        const messages: string[] = [];
        if (result.groups_imported > 0) messages.push(`${result.groups_imported} server group(s) imported`);
        if (result.forwardings_imported > 0) messages.push(`${result.forwardings_imported} forwarding(s) imported`);
        if (result.groups_skipped > 0) messages.push(`${result.groups_skipped} server group(s) skipped`);
        if (result.forwardings_skipped > 0) messages.push(`${result.forwardings_skipped} forwarding(s) skipped`);
        if (result.errors.length > 0) messages.push(`Warnings: ${result.errors.join(', ')}`);
        if (result.success) {
          alert(`Import completed:\n${messages.join('\n')}`);
        } else {
          alert(`Import failed:\n${result.errors.join('\n')}`);
        }
      }
    } catch (e) {
      alert(`Failed to import configs: ${e instanceof Error ? e.message : e}`);
    } finally {
      e.target.value = '';
    }
  }, [refreshGroups, refreshForwardings, performActionWithRecipient]);

  // TOML export handler
  const handleExportForwardingToml = useCallback(async (id: string) => {
    try {
      const recipient = await resolveRecipient({ type: 'toml-export', forwardingId: id });
      if (recipient) {
        await performActionWithRecipient(recipient, { type: 'toml-export', forwardingId: id });
      }
    } catch (e) {
      alert(`Failed to export forwarding config: ${e instanceof Error ? e.message : e}`);
    }
  }, [resolveRecipient, performActionWithRecipient]);

  // Age key dialog callbacks
  const handleAgeKeyComplete = useCallback(async (recipient: string) => {
    setShowAgeKeyDialog(false);
    const action = pendingAction;
    setPendingAction(null);
    if (action) {
      if (action.type === 'import') {
        // Import doesn't use recipient for encryption, just needs key to exist for decryption
        await performActionWithRecipient(recipient, action);
      } else {
        await performActionWithRecipient(recipient, action);
      }
    }
  }, [pendingAction, performActionWithRecipient]);

  const handleAgeKeyCancel = useCallback(() => {
    setShowAgeKeyDialog(false);
    setPendingAction(null);
  }, []);

  const handleAgeKeyGenerate = useCallback(async () => {
    return await invoke<string>('generate_age_key');
  }, []);

  // Scroll a group card into view in the main content area
  const scrollToGroupCard = useCallback((groupId: string) => {
    if (view !== 'list') return;
    requestAnimationFrame(() => {
      const el = document.getElementById(`group-card-${groupId}`);
      el?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  }, [view]);

  // Sidebar selection handlers
  const handleSelectGroup = useCallback((id: string) => {
    setSelectedGroupId(id);
    setSelectedForwardingId(null);
    scrollToGroupCard(id);
  }, [scrollToGroupCard]);

  const handleSelectForwarding = useCallback((id: string) => {
    const forwarding = getForwarding(id);
    if (forwarding) {
      setSelectedGroupId(forwarding.server_group_id);
      setSelectedForwardingId(id);
      scrollToGroupCard(forwarding.server_group_id);
    }
  }, [getForwarding, scrollToGroupCard]);

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
  const runningCount = instances.filter(i => i.status === 'running' || i.status === 'starting' || i.status === 'reconnecting').length;

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

      <main className="main-content" ref={mainContentRef}>
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
              <div className="export-import-row">
                <button
                  className="btn-small"
                  onClick={handleExport}
                  title="Export all configs with age-encrypted credentials"
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
                    onExportForwardingToml={handleExportForwardingToml}
                    loading={instancesLoading}
                  />
                ))}
              </div>
            )}

            <div className="binary-path-row">
              <span className="binary-path-info" title={isUsingBundled ? 'Bundled' : (customBinaryPath ?? 'Not set')}>
                tunnel-rs binary: {isUsingBundled ? 'Bundled' : (customBinaryPath ?? 'Not set')}{binaryVersion ? ` (${binaryVersion})` : ''}
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
          </>
        )}
      </main>

      {pendingDelete && (
        <ConfirmDialog
          message={`Are you sure you want to delete "${pendingDelete.name}"?`}
          onConfirm={handleConfirmDelete}
          onCancel={handleCancelDelete}
          loading={deleting}
        />
      )}

      {showAgeKeyDialog && (
        <AgeKeyDialog
          mode={ageKeyMode}
          recipients={ageKeyRecipients}
          onComplete={handleAgeKeyComplete}
          onCancel={handleAgeKeyCancel}
          onGenerate={handleAgeKeyGenerate}
        />
      )}
    </div>
  );
}

export default App;
