<template>
  <div class="app-container">
    <!-- Header -->
    <header class="app-header">
      <div class="header-left">
        <h1>👻 GhostHandDesk</h1>
      </div>
      <div class="header-center">
        <div class="device-id-display">
          <span class="label">Device ID:</span>
          <code class="device-id">{{ deviceId || 'Chargement...' }}</code>
          <button
            v-if="deviceId"
            @click="copyDeviceId"
            class="copy-btn"
            title="Copier"
          >
            📋
          </button>
        </div>
      </div>
      <div class="header-right">
        <div class="connection-status" :class="statusClass">
          <span class="status-indicator"></span>
          <span>{{ statusText }}</span>
        </div>
        <button
          v-if="showSettings"
          @click="settingsOpen = !settingsOpen"
          class="settings-btn"
        >
          ⚙️
        </button>
      </div>
    </header>

    <!-- Main content -->
    <main class="app-main">
      <!-- Connect Dialog (non connecté) -->
      <ConnectDialog
        v-if="!connected"
        @connect="handleConnect"
        :connecting="connecting"
        :error="connectionError"
      />

      <!-- Remote Viewer (connecté) -->
      <RemoteViewer
        v-else
        :connection-id="connectedTo"
        @disconnect="handleDisconnect"
      />
    </main>

    <!-- Settings Panel (overlay) -->
    <SettingsPanel
      v-if="settingsOpen"
      @close="settingsOpen = false"
      @update="handleSettingsUpdate"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import ConnectDialog from './components/ConnectDialog.vue';
import RemoteViewer from './components/RemoteViewer.vue';
import SettingsPanel from './components/SettingsPanel.vue';

// États réactifs
const deviceId = ref<string>('');
const connected = ref(false);
const connecting = ref(false);
const connectedTo = ref<string>('');
const connectionError = ref<string>('');
const settingsOpen = ref(false);
const showSettings = ref(true);

// Computed properties
const statusClass = computed(() => {
  if (connected.value) return 'status-connected';
  if (connecting.value) return 'status-connecting';
  return 'status-disconnected';
});

const statusText = computed(() => {
  if (connected.value) return `Connecté à ${connectedTo.value}`;
  if (connecting.value) return 'Connexion...';
  return 'Déconnecté';
});

// Lifecycle
onMounted(async () => {
  try {
    // Récupérer le Device ID depuis le backend Rust
    deviceId.value = await invoke<string>('get_device_id');
    console.log('Device ID:', deviceId.value);
  } catch (error) {
    console.error('Erreur récupération Device ID:', error);
    connectionError.value = 'Impossible de récupérer le Device ID';
  }
});

// Méthodes
async function handleConnect(targetId: string, password: string | null) {
  connecting.value = true;
  connectionError.value = '';

  try {
    console.log('Tentative de connexion à:', targetId);

    await invoke('connect_to_device', {
      targetId,
      password: password || undefined,
    });

    connected.value = true;
    connectedTo.value = targetId;
    console.log('Connexion établie !');
  } catch (error: any) {
    console.error('Erreur de connexion:', error);
    connectionError.value = error.message || 'Échec de la connexion';
    connected.value = false;
  } finally {
    connecting.value = false;
  }
}

async function handleDisconnect() {
  try {
    await invoke('disconnect');
    connected.value = false;
    connectedTo.value = '';
    console.log('Déconnecté');
  } catch (error) {
    console.error('Erreur de déconnexion:', error);
  }
}

function copyDeviceId() {
  if (deviceId.value) {
    navigator.clipboard.writeText(deviceId.value);
    // Optionnel: afficher une notification
    console.log('Device ID copié !');
  }
}

function handleSettingsUpdate(settings: any) {
  console.log('Mise à jour paramètres:', settings);
  // TODO: Envoyer au backend
}
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #1e1e1e;
}

/* Header */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  background: #2d2d30;
  border-bottom: 1px solid #3e3e42;
  height: 60px;
}

.header-left h1 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.header-center {
  flex: 1;
  display: flex;
  justify-content: center;
}

.device-id-display {
  display: flex;
  align-items: center;
  gap: 10px;
}

.device-id-display .label {
  color: #9d9d9d;
  font-size: 13px;
}

.device-id {
  background: #1e1e1e;
  padding: 6px 12px;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 14px;
  color: #4ec9b0;
  border: 1px solid #3e3e42;
}

.copy-btn {
  background: transparent;
  border: none;
  font-size: 16px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.2s;
}

.copy-btn:hover {
  background: #3e3e42;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 15px;
}

/* Status */
.connection-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 13px;
}

.status-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-disconnected .status-indicator {
  background: #666;
}

.status-connecting .status-indicator {
  background: #ffa500;
  animation: pulse 1.5s infinite;
}

.status-connected .status-indicator {
  background: #4ec9b0;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.settings-btn {
  background: transparent;
  border: none;
  font-size: 20px;
  cursor: pointer;
  padding: 6px 10px;
  border-radius: 4px;
  transition: background 0.2s;
}

.settings-btn:hover {
  background: #3e3e42;
}

/* Main */
.app-main {
  flex: 1;
  overflow: hidden;
  position: relative;
}
</style>
