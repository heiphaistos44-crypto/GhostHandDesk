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
        <div v-if="networkInfo.local_ip" class="network-info">
          <span class="label">IP:</span>
          <code class="network-ip">{{ networkInfo.local_ip }}:{{ networkInfo.port }}</code>
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
        @cancel="handleCancelConnect"
        @server-changed="handleServerChanged"
        :connecting="connecting"
        :error="connectionError"
      />

      <!-- Remote Viewer (contrôleur - celui qui a initié la connexion) -->
      <RemoteViewer
        v-else-if="!isControlled"
        :connection-id="connectedTo"
        @disconnect="handleDisconnect"
      />

      <!-- Écran "contrôlé" avec preview (celui qui a accepté la connexion) -->
      <div v-else class="controlled-view">
        <canvas ref="previewCanvasRef" class="preview-canvas"></canvas>
        <div v-if="!previewActive" class="preview-waiting">
          <p>Démarrage du preview...</p>
        </div>
        <div class="controlled-controls">
          <div class="controlled-badge">
            🖥️ <code>{{ connectedTo }}</code> contrôle cet appareil
          </div>
          <button @click="handleDisconnect" class="disconnect-btn-floating">
            Arrêter le partage
          </button>
        </div>
      </div>
    </main>

    <!-- Settings Panel (overlay) -->
    <SettingsPanel
      v-if="settingsOpen"
      @close="settingsOpen = false"
      @update="handleSettingsUpdate"
    />

    <!-- Connection Request Dialog (popup demande entrante) -->
    <ConnectionRequestDialog
      :visible="connectionRequestVisible"
      :request-from="pendingRequest.from"
      :timestamp="pendingRequest.timestamp"
      @accept="handleAcceptRequest"
      @reject="handleRejectRequest"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import ConnectDialog from './components/ConnectDialog.vue';
import RemoteViewer from './components/RemoteViewer.vue';
import SettingsPanel from './components/SettingsPanel.vue';
import ConnectionRequestDialog from './components/ConnectionRequestDialog.vue';

// Types
interface ConnectionRequest {
  from: string;
  timestamp: number;
}

// États réactifs
const deviceId = ref<string>('');
const connected = ref(false);
const connecting = ref(false);
const connectedTo = ref<string>('');
const connectionError = ref<string>('');
const settingsOpen = ref(false);
const showSettings = ref(true);
const isControlled = ref(false);
const networkInfo = ref<{local_ip: string; port: string; server_url: string}>({
  local_ip: '',
  port: '9000',
  server_url: '',
});

// Preview local (PC contrôlé)
const previewCanvasRef = ref<HTMLCanvasElement | null>(null);
const previewActive = ref(false);
let previewHandler: ((event: Event) => void) | null = null;

// États pour la popup de demande de connexion
const connectionRequestVisible = ref(false);
const pendingRequest = ref<ConnectionRequest>({
  from: '',
  timestamp: 0
});

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

// Preview local : écouter les frames quand contrôlé
watch(isControlled, (val) => {
  if (val) {
    // Setup le listener pour les frames du preview local
    previewHandler = ((event: Event) => {
      const canvas = previewCanvasRef.value;
      if (!canvas) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      const detail = (event as CustomEvent).detail;
      const binaryString = atob(detail.data);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      const blob = new Blob([bytes], { type: 'image/jpeg' });
      createImageBitmap(blob).then((bmp) => {
        // Adapter le canvas au container
        const parent = canvas.parentElement;
        if (parent) {
          canvas.width = parent.clientWidth;
          canvas.height = parent.clientHeight;
        }
        // Dessiner avec ratio préservé
        const scale = Math.min(canvas.width / bmp.width, canvas.height / bmp.height);
        const dw = bmp.width * scale;
        const dh = bmp.height * scale;
        const dx = (canvas.width - dw) / 2;
        const dy = (canvas.height - dh) / 2;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(bmp, dx, dy, dw, dh);
        bmp.close();
        if (!previewActive.value) previewActive.value = true;
      }).catch(() => {});
    });
    window.addEventListener('ghosthand-local-preview', previewHandler);
  } else {
    // Cleanup
    if (previewHandler) {
      window.removeEventListener('ghosthand-local-preview', previewHandler);
      previewHandler = null;
    }
    previewActive.value = false;
  }
});

onUnmounted(() => {
  if (previewHandler) {
    window.removeEventListener('ghosthand-local-preview', previewHandler);
  }
});

// Lifecycle
onMounted(async () => {
  try {
    // Récupérer le Device ID depuis le backend Rust
    deviceId.value = await invoke<string>('get_device_id');

    // Récupérer les infos réseau
    try {
      networkInfo.value = await invoke<any>('get_network_info');
    } catch (e) {
      console.error('Erreur récupération infos réseau:', e);
    }

    // Initialiser la session au démarrage
    await invoke('initialize_session');

    // Démarrer l'écoute des demandes de connexion entrantes
    await invoke('start_listening_for_requests');

    // Écouter les demandes de connexion via DOM CustomEvent
    // (window.eval() + CustomEvent car le Tauri event system ne fonctionne pas)
    window.addEventListener('ghosthand-connect-request', ((event: CustomEvent) => {
      console.log('[APP] Demande de connexion reçue:', event.detail);
      pendingRequest.value = event.detail;
      connectionRequestVisible.value = true;
    }) as EventListener);
    console.log('[APP] Listener connexion enregistré');

  } catch (error) {
    console.error('Erreur initialisation:', error);
    connectionError.value = 'Impossible d\'initialiser l\'application';
  }
});

// Méthodes
async function handleConnect(targetId: string, password: string | null) {
  connecting.value = true;
  connectionError.value = '';

  try {

    await invoke('connect_to_device', {
      targetId,
      password: password || undefined,
    });

    connected.value = true;
    connectedTo.value = targetId;

    // Auto-démarrer la réception vidéo
    try {
      await invoke('start_receiving');
    } catch (error) {
      console.error('Erreur démarrage réception:', error);
    }

  } catch (error: any) {
    console.error('Erreur de connexion:', error);
    connectionError.value = error.message || 'Échec de la connexion';
    connected.value = false;
  } finally {
    connecting.value = false;
  }
}

async function handleCancelConnect() {
  console.log('[APP] Annulation de la connexion');
  connecting.value = false;
  connectionError.value = '';
  try {
    await invoke('disconnect');
  } catch (e) {
    // Silencieux - la session était peut-être déjà fermée
  }
}

async function handleAcceptRequest() {
  connectionRequestVisible.value = false;

  try {

    await invoke('accept_connection', {
      from: pendingRequest.value.from
    });

    connected.value = true;
    connectedTo.value = pendingRequest.value.from;
    isControlled.value = true;

    // Auto-démarrer le streaming et l'input handler
    try {
      await invoke('start_streaming');

      await invoke('start_input_handler');
    } catch (error) {
      console.error('Erreur démarrage streaming/input:', error);
    }

  } catch (error: any) {
    console.error('Erreur acceptation connexion:', error);
    connectionError.value = error.message || 'Échec de l\'acceptation';
  }
}

async function handleRejectRequest() {
  connectionRequestVisible.value = false;

  try {

    await invoke('reject_connection', {
      from: pendingRequest.value.from,
      reason: 'Refusé par l\'utilisateur'
    });

  } catch (error) {
    console.error('Erreur rejet connexion:', error);
  }
}

async function handleDisconnect() {
  try {
    await invoke('disconnect');
    connected.value = false;
    connectedTo.value = '';
    isControlled.value = false;
  } catch (error) {
    console.error('Erreur de déconnexion:', error);
  }
}

function copyDeviceId() {
  if (deviceId.value) {
    navigator.clipboard.writeText(deviceId.value);
    // Optionnel: afficher une notification
  }
}

async function handleServerChanged(serverUrl: string) {
  console.log('[APP] Serveur changé:', serverUrl);

  // Mettre à jour l'affichage réseau
  try {
    networkInfo.value = await invoke<any>('get_network_info');
  } catch (e) {
    console.error('Erreur récupération infos réseau:', e);
  }

  // Relancer l'écoute des demandes de connexion sur le nouveau serveur
  try {
    await invoke('start_listening_for_requests');
    console.log('[APP] Listener relancé sur nouveau serveur');
  } catch (e) {
    console.error('Erreur relance listener:', e);
  }
}

async function handleSettingsUpdate(settings: any) {
  try {
    // Mapper les settings UI vers le format Config Rust
    const newConfig = {
      server_url: settings.serverUrl,
      stun_servers: settings.stunServers,
      turn_servers: [],
      video_config: {
        framerate: settings.framerate,
        codec: settings.codec,
        bitrate: settings.bitrate,
        hardware_acceleration: settings.hardwareAcceleration,
        resolution: null,
      },
      network_config: {
        max_packet_size: 65536,
        connection_timeout: 30,
        enable_ipv6: true,
      },
      security_config: {
        e2e_encryption: settings.encryptData,
        require_auth: settings.requirePassword,
        cert_path: null,
        password_hash: null,
      },
    };

    await invoke('update_config', { newConfig });
    console.log('[APP] Configuration mise à jour avec succès');
  } catch (error) {
    console.error('Erreur mise à jour settings:', error);
  }
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

/* Écran contrôlé - preview */
.controlled-view {
  position: relative;
  height: 100%;
  background: #000;
}

.preview-canvas {
  width: 100%;
  height: 100%;
  display: block;
}

.preview-waiting {
  position: absolute;
  inset: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  color: #888;
  font-size: 14px;
}

.controlled-controls {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 15px;
  padding: 10px 20px;
  background: rgba(0, 0, 0, 0.75);
  border-radius: 10px;
  backdrop-filter: blur(8px);
}

.controlled-badge {
  font-size: 13px;
  color: #ccc;
  display: flex;
  align-items: center;
  gap: 8px;
}

.controlled-badge code {
  color: #4ec9b0;
  font-family: monospace;
  background: rgba(0, 0, 0, 0.4);
  padding: 2px 8px;
  border-radius: 3px;
}

.disconnect-btn-floating {
  padding: 8px 20px;
  background: #c44;
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
}

.disconnect-btn-floating:hover {
  background: #e55;
}
</style>
