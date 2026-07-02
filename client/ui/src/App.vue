<template>
  <div class="app-container">
    <!-- Toast copie -->
    <Transition name="toast">
      <div v-if="copyToast" class="copy-toast">Copié !</div>
    </Transition>

    <!-- Header -->
    <header class="app-header">
      <div class="header-left">
        <span class="app-logo">👻</span>
        <h1>GhostHandDesk</h1>
      </div>

      <div class="header-center">
        <div class="device-id-display">
          <span class="label">ID:</span>
          <code class="device-id">{{ deviceId || '…' }}</code>
          <button v-if="deviceId" @click="copyDeviceId" class="copy-btn" title="Copier ID">
            📋
          </button>
        </div>
        <div v-if="networkInfo.local_ip" class="network-info">
          <span class="net-label">{{ networkInfo.local_ip }}</span>
        </div>
      </div>

      <div class="header-right">
        <!-- Statistiques système -->
        <div class="sys-stats">
          <!-- CPU -->
          <div class="stat-pill" :class="cpuClass" title="Utilisation CPU">
            <span class="stat-icon">⚡</span>
            <div class="stat-content">
              <span class="stat-label">CPU</span>
              <span class="stat-value">{{ sysStats.cpu_usage.toFixed(1) }}%</span>
            </div>
            <div class="stat-bar">
              <div class="stat-bar-fill" :style="{ width: sysStats.cpu_usage + '%' }"></div>
            </div>
          </div>

          <!-- RAM -->
          <div class="stat-pill" :class="ramClass" title="Mémoire RAM">
            <span class="stat-icon">🧠</span>
            <div class="stat-content">
              <span class="stat-label">RAM</span>
              <span class="stat-value">{{ formatGB(sysStats.ram_used) }}/{{ formatGB(sysStats.ram_total) }}</span>
            </div>
            <div class="stat-bar">
              <div class="stat-bar-fill" :style="{ width: ramPercent + '%' }"></div>
            </div>
          </div>

          <!-- Disque -->
          <div class="stat-pill" :class="diskClass" title="Disque C:\">
            <span class="stat-icon">💾</span>
            <div class="stat-content">
              <span class="stat-label">Disk</span>
              <span class="stat-value">{{ formatGB(sysStats.disk_used) }}/{{ formatGB(sysStats.disk_total) }}</span>
            </div>
            <div class="stat-bar">
              <div class="stat-bar-fill" :style="{ width: diskPercent + '%' }"></div>
            </div>
          </div>

          <!-- Uptime -->
          <div class="stat-pill uptime-pill" title="Uptime système">
            <span class="stat-icon">⏱</span>
            <div class="stat-content">
              <span class="stat-label">Uptime</span>
              <span class="stat-value">{{ formatUptime(sysStats.uptime_secs) }}</span>
            </div>
          </div>
        </div>

        <!-- Séparateur -->
        <div class="header-sep"></div>

        <!-- Horloge -->
        <div class="clock-pill">{{ currentTime }}</div>

        <!-- Séparateur -->
        <div class="header-sep"></div>

        <!-- Statut connexion -->
        <div class="connection-status" :class="statusClass" :title="statusText">
          <span class="status-indicator"></span>
          <span class="status-text">{{ statusText }}</span>
        </div>

        <!-- Settings -->
        <button
          v-if="showSettings"
          @click="settingsOpen = !settingsOpen"
          class="settings-btn"
          title="Paramètres"
        >
          ⚙️
        </button>
      </div>
    </header>

    <!-- Bandeau sécurité E2E : empreinte de session (SAS) à comparer -->
    <div v-if="connected && sessionFingerprint" class="secure-bar" :class="{ authed: sessionAuthenticated }">
      <span class="secure-lock">🔒</span>
      <span class="secure-text">
        Session chiffrée · Empreinte <code class="secure-fp">{{ sessionFingerprint }}</code>
        <template v-if="sessionAuthenticated"> · authentifiée par mot de passe</template>
        <template v-else> · comparez cette empreinte avec l'autre poste pour écarter tout intercepteur</template>
      </span>
    </div>

    <!-- Main content -->
    <main class="app-main">
      <ConnectDialog
        v-if="!connected"
        @connect="handleConnect"
        @cancel="handleCancelConnect"
        @server-changed="handleServerChanged"
        @open-settings="settingsOpen = true"
        :connecting="connecting"
        :error="connectionError"
      />

      <RemoteViewer
        v-else-if="!isControlled"
        :connection-id="connectedTo"
        :device-id="deviceId"
        @disconnect="handleDisconnect"
      />

      <div v-else class="controlled-view">
        <canvas ref="previewCanvasRef" class="preview-canvas"></canvas>
        <div v-if="!previewActive" class="preview-waiting">
          <div class="spinner"></div>
          <p>Démarrage du preview…</p>
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

    <!-- Connection Request Dialog -->
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
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import ConnectDialog from './components/ConnectDialog.vue';
import RemoteViewer from './components/RemoteViewer.vue';
import SettingsPanel from './components/SettingsPanel.vue';
import ConnectionRequestDialog from './components/ConnectionRequestDialog.vue';

interface ConnectionRequest {
  from: string;
  timestamp: number;
}

interface SystemStats {
  cpu_usage: number;
  ram_used: number;
  ram_total: number;
  disk_used: number;
  disk_total: number;
  uptime_secs: number;
}

// États réactifs
const deviceId = ref('');
const connected = ref(false);
const connecting = ref(false);
const connectedTo = ref('');
const connectionError = ref('');
const settingsOpen = ref(false);
const showSettings = ref(true);
const isControlled = ref(false);
const copyToast = ref(false);
const networkInfo = ref<{ local_ip: string; port: string; server_url: string }>({
  local_ip: '',
  port: '9000',
  server_url: '',
});
const currentServerUrl = ref('');

// Statistiques système
const sysStats = ref<SystemStats>({
  cpu_usage: 0,
  ram_used: 0,
  ram_total: 0,
  disk_used: 0,
  disk_total: 0,
  uptime_secs: 0,
});

// Horloge
const currentTime = ref('');

// Preview local (PC contrôlé)
const previewCanvasRef = ref<HTMLCanvasElement | null>(null);
const previewActive = ref(false);
let previewUnlisten: UnlistenFn | null = null;
let connectRequestUnlisten: UnlistenFn | null = null;
let streamingErrorUnlisten: UnlistenFn | null = null;
let sessionSecureUnlisten: UnlistenFn | null = null;

// Sécurité E2E : empreinte de session (SAS) à comparer hors-bande
const sessionFingerprint = ref('');
const sessionAuthenticated = ref(false);

// Popup connexion entrante
const connectionRequestVisible = ref(false);
const pendingRequest = ref<ConnectionRequest>({ from: '', timestamp: 0 });

// Intervalles
let statsInterval: ReturnType<typeof setInterval> | null = null;
let clockInterval: ReturnType<typeof setInterval> | null = null;

// Computed
const statusClass = computed(() => {
  if (connected.value) return 'status-connected';
  if (connecting.value) return 'status-connecting';
  return 'status-disconnected';
});

const statusText = computed(() => {
  if (connected.value) return `Connecté · ${connectedTo.value}`;
  if (connecting.value) return 'Connexion…';
  return 'Déconnecté';
});

const ramPercent = computed(() => {
  if (!sysStats.value.ram_total) return 0;
  return Math.round((sysStats.value.ram_used / sysStats.value.ram_total) * 100);
});

const diskPercent = computed(() => {
  if (!sysStats.value.disk_total) return 0;
  return Math.round((sysStats.value.disk_used / sysStats.value.disk_total) * 100);
});

const cpuClass = computed(() => {
  const v = sysStats.value.cpu_usage;
  if (v >= 85) return 'stat-critical';
  if (v >= 60) return 'stat-warn';
  return 'stat-ok';
});

const ramClass = computed(() => {
  const v = ramPercent.value;
  if (v >= 90) return 'stat-critical';
  if (v >= 70) return 'stat-warn';
  return 'stat-ok';
});

const diskClass = computed(() => {
  const v = diskPercent.value;
  if (v >= 95) return 'stat-critical';
  if (v >= 80) return 'stat-warn';
  return 'stat-ok';
});

// Formatage
function formatGB(bytes: number): string {
  if (!bytes) return '0 GB';
  const gb = bytes / (1024 ** 3);
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${(bytes / (1024 ** 2)).toFixed(0)} MB`;
}

function formatUptime(secs: number): string {
  if (!secs) return '—';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h >= 24) {
    const d = Math.floor(h / 24);
    return `${d}j ${h % 24}h`;
  }
  return `${h}h ${m}m`;
}

function updateClock() {
  const now = new Date();
  currentTime.value = now.toLocaleTimeString('fr-FR', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

async function fetchSystemStats() {
  try {
    const stats = await invoke<SystemStats>('get_system_stats');
    sysStats.value = stats;
  } catch (e) {
    // Silencieux si non dispo
  }
}

// Preview local
interface LocalPreviewPayload {
  data: string;
  width: number;
  height: number;
  timestamp: number;
}

watch(isControlled, (val) => {
  if (val) {
    listen<LocalPreviewPayload>('ghosthand-local-preview', (event) => {
      const canvas = previewCanvasRef.value;
      if (!canvas) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      const detail = event.payload;
      const binaryString = atob(detail.data);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      const blob = new Blob([bytes], { type: 'image/jpeg' });
      createImageBitmap(blob).then((bmp) => {
        const parent = canvas.parentElement;
        if (parent) {
          canvas.width = parent.clientWidth;
          canvas.height = parent.clientHeight;
        }
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
    }).then((unlisten) => {
      previewUnlisten = unlisten;
    });
  } else {
    previewUnlisten?.();
    previewUnlisten = null;
    previewActive.value = false;
  }
});

onUnmounted(() => {
  previewUnlisten?.();
  connectRequestUnlisten?.();
  streamingErrorUnlisten?.();
  sessionSecureUnlisten?.();
  if (statsInterval) clearInterval(statsInterval);
  if (clockInterval) clearInterval(clockInterval);
});

onMounted(async () => {
  // Horloge immédiate
  updateClock();
  clockInterval = setInterval(updateClock, 1000);

  // Stats système (premier fetch immédiat + polling toutes les 2s)
  await fetchSystemStats();
  statsInterval = setInterval(fetchSystemStats, 2000);

  try {
    deviceId.value = await invoke<string>('get_device_id');

    try {
      networkInfo.value = await invoke<any>('get_network_info');
      currentServerUrl.value = networkInfo.value.server_url || '';
    } catch (e) {
      console.error('Erreur réseau:', e);
    }

    await invoke('initialize_session');
    await invoke('start_listening_for_requests');

    streamingErrorUnlisten = await listen<string>('ghosthand-streaming-error', (event) => {
      console.error('[APP]', event.payload);
    });

    connectRequestUnlisten = await listen<ConnectionRequest>('ghosthand-connect-request', (event) => {
      pendingRequest.value = event.payload;
      connectionRequestVisible.value = true;
    });

    sessionSecureUnlisten = await listen<{ fingerprint: string; authenticated: boolean }>(
      'ghosthand-session-secure',
      (event) => {
        sessionFingerprint.value = event.payload.fingerprint;
        sessionAuthenticated.value = event.payload.authenticated;
      }
    );
  } catch (error) {
    console.error('Erreur init:', error);
    connectionError.value = 'Impossible d\'initialiser l\'application';
  }
});

// Méthodes
async function handleConnect(targetId: string, password: string | null) {
  connecting.value = true;
  connectionError.value = '';

  try {
    await invoke('connect_to_device', { targetId, password: password || undefined });
    connected.value = true;
    connectedTo.value = targetId;

    try {
      await invoke('start_receiving');
    } catch (error) {
      console.error('Erreur démarrage réception:', error);
    }
  } catch (error: any) {
    // Tauri retourne une String (pas un Error object) → error.message est undefined
    connectionError.value = typeof error === 'string' ? error : (error?.message || String(error) || 'Échec de la connexion');
    connected.value = false;
  } finally {
    connecting.value = false;
  }
}

async function handleCancelConnect() {
  connecting.value = false;
  connectionError.value = '';
  try {
    await invoke('disconnect');
  } catch (e) {}
}

async function handleAcceptRequest() {
  connectionRequestVisible.value = false;

  try {
    await invoke('accept_connection', { from: pendingRequest.value.from });
    connected.value = true;
    connectedTo.value = pendingRequest.value.from;
    isControlled.value = true;

    try {
      await invoke('start_streaming');
      await invoke('start_input_handler');
    } catch (error) {
      console.error('Erreur streaming/input:', error);
    }
  } catch (error: any) {
    connectionError.value = typeof error === 'string' ? error : (error?.message || String(error) || 'Échec de l\'acceptation');
  }
}

async function handleRejectRequest() {
  connectionRequestVisible.value = false;
  try {
    await invoke('reject_connection', { from: pendingRequest.value.from, reason: 'Refusé par l\'utilisateur' });
  } catch (error) {
    console.error('Erreur rejet:', error);
  }
}

async function handleDisconnect() {
  try {
    await invoke('stop_streaming').catch(() => {});
    await invoke('disconnect');
    connected.value = false;
    connectedTo.value = '';
    isControlled.value = false;
    connectionError.value = '';
    sessionFingerprint.value = '';
    sessionAuthenticated.value = false;
    // Recréer la session pour permettre de nouvelles connexions
    try {
      await invoke('initialize_session');
      await invoke('start_listening_for_requests');
    } catch (e) {
      console.error('Erreur réinitialisation session:', e);
      connectionError.value = 'Reconnexion au serveur échouée. Redémarrez l\'application.';
    }
  } catch (error) {
    console.error('Erreur déconnexion:', error);
  }
}

async function copyDeviceId() {
  if (!deviceId.value) return;
  try {
    await navigator.clipboard.writeText(deviceId.value);
    copyToast.value = true;
    setTimeout(() => { copyToast.value = false; }, 2000);
  } catch (e) {
    console.error('Erreur copie:', e);
  }
}

async function handleServerChanged(_serverUrl: string) {
  try {
    networkInfo.value = await invoke<any>('get_network_info');
  } catch (e) {}

  try {
    await invoke('start_listening_for_requests');
  } catch (e) {
    console.error('Erreur relance listener:', e);
  }
}

async function handleSettingsUpdate(settings: any) {
  try {
    // 1. Persister server_url + stun_servers sur disque
    await invoke('save_settings', {
      settings: {
        server_url: settings.serverUrl,
        stun_servers: settings.stunServers,
      },
    });

    // 2. Mettre à jour le reste de la config en mémoire (codec, bitrate, etc.)
    await invoke('update_config', {
      newConfig: {
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
      },
    });

    // 3. Définir / effacer le mot de passe de l'appareil
    const pwd = settings.requirePassword && settings.connectionPassword
      ? settings.connectionPassword
      : null;
    await invoke('set_device_password', { password: pwd });

    // 4. Reconnecter si l'URL du serveur a changé
    if (settings.serverUrl && settings.serverUrl !== currentServerUrl.value) {
      await invoke('update_server_url', { serverUrl: settings.serverUrl });
      currentServerUrl.value = settings.serverUrl;
    }
  } catch (error) {
    console.error('Erreur settings:', error);
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
  overflow: hidden;
}

/* ─── Toast copie ─── */
.copy-toast {
  position: fixed;
  top: 70px;
  left: 50%;
  transform: translateX(-50%);
  background: #4ec9b0;
  color: #000;
  padding: 6px 18px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  z-index: 9999;
  pointer-events: none;
}
.toast-enter-active, .toast-leave-active { transition: opacity 0.2s, transform 0.2s; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateX(-50%) translateY(-8px); }

/* ─── Header ─── */
.app-header {
  display: flex;
  align-items: center;
  padding: 0 16px;
  background: #2d2d30;
  border-bottom: 1px solid #3e3e42;
  height: 54px;
  flex-shrink: 0;
  gap: 12px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.app-logo { font-size: 22px; }

.header-left h1 {
  font-size: 17px;
  font-weight: 700;
  margin: 0;
  white-space: nowrap;
  color: #e0e0e0;
  letter-spacing: -0.3px;
}

.header-center {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.device-id-display {
  display: flex;
  align-items: center;
  gap: 6px;
}

.label {
  color: #777;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.device-id {
  background: #1a1a1a;
  padding: 4px 10px;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #4ec9b0;
  border: 1px solid #3a3a3a;
  letter-spacing: 0.5px;
}

.copy-btn {
  background: transparent;
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 3px 6px;
  border-radius: 4px;
  transition: background 0.15s;
  line-height: 1;
}

.copy-btn:hover { background: #3e3e42; }

.network-info {
  display: flex;
  align-items: center;
}

.net-label {
  font-size: 11px;
  color: #666;
  font-family: monospace;
  background: #1a1a1a;
  padding: 3px 8px;
  border-radius: 3px;
  border: 1px solid #333;
}

/* ─── Right side ─── */
.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}

.header-sep {
  width: 1px;
  height: 28px;
  background: #3e3e42;
  flex-shrink: 0;
}

/* ─── Stats système ─── */
.sys-stats {
  display: flex;
  align-items: center;
  gap: 6px;
}

.stat-pill {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  background: #1a1a1a;
  border: 1px solid #333;
  border-radius: 6px;
  min-width: 90px;
  position: relative;
  overflow: hidden;
  transition: border-color 0.3s;
}

.stat-pill.stat-ok    { border-color: #2a5a4a; }
.stat-pill.stat-warn  { border-color: #5a4a1a; }
.stat-pill.stat-critical { border-color: #5a2020; }

.stat-icon {
  font-size: 13px;
  flex-shrink: 0;
}

.stat-content {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.stat-label {
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #666;
  line-height: 1;
}

.stat-value {
  font-size: 11px;
  font-family: monospace;
  color: #ccc;
  line-height: 1.3;
  white-space: nowrap;
}

.stat-pill.stat-ok    .stat-value { color: #4ec9b0; }
.stat-pill.stat-warn  .stat-value { color: #ffa500; }
.stat-pill.stat-critical .stat-value { color: #ff6666; }

/* Barre de progression mini */
.stat-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: #2a2a2a;
}

.stat-bar-fill {
  height: 100%;
  background: #4ec9b0;
  transition: width 0.5s ease;
}

.stat-pill.stat-warn  .stat-bar-fill { background: #ffa500; }
.stat-pill.stat-critical .stat-bar-fill { background: #ff6666; }

/* Uptime sans barre */
.uptime-pill {
  min-width: 80px;
}

/* ─── Horloge ─── */
.clock-pill {
  font-size: 13px;
  font-family: 'Courier New', monospace;
  color: #9d9d9d;
  background: #1a1a1a;
  border: 1px solid #333;
  padding: 4px 10px;
  border-radius: 6px;
  white-space: nowrap;
  letter-spacing: 0.5px;
}

/* ─── Statut connexion ─── */
.connection-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 12px;
  background: #1a1a1a;
  border: 1px solid #333;
  white-space: nowrap;
  max-width: 180px;
  overflow: hidden;
}

.status-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-indicator {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-disconnected .status-indicator { background: #555; }
.status-connecting   .status-indicator { background: #ffa500; animation: pulse 1.5s infinite; }
.status-connected    .status-indicator { background: #4ec9b0; box-shadow: 0 0 4px #4ec9b0; }

/* Bandeau sécurité E2E (empreinte de session) */
.secure-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  font-size: 12px;
  background: #3a2e12;
  color: #e6c200;
  border-bottom: 1px solid #5a4a1a;
}
.secure-bar.authed {
  background: #122e1a;
  color: #4ec9b0;
  border-bottom-color: #1a5a3a;
}
.secure-lock { font-size: 13px; }
.secure-fp {
  font-family: 'Consolas', 'Courier New', monospace;
  font-weight: 700;
  letter-spacing: 1px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* ─── Bouton settings ─── */
.settings-btn {
  background: transparent;
  border: 1px solid #3e3e42;
  font-size: 18px;
  cursor: pointer;
  padding: 5px 9px;
  border-radius: 6px;
  transition: background 0.2s;
  line-height: 1;
}

.settings-btn:hover { background: #3e3e42; }

/* ─── Main ─── */
.app-main {
  flex: 1;
  overflow: hidden;
  position: relative;
  min-height: 0;
}

/* ─── Écran contrôlé ─── */
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
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #888;
  gap: 16px;
}

.spinner {
  width: 36px;
  height: 36px;
  border: 3px solid rgba(255,255,255,0.1);
  border-top-color: #4ec9b0;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.preview-waiting p { font-size: 14px; margin: 0; }

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
  border: 1px solid rgba(255,255,255,0.08);
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
  background: rgba(0,0,0,0.4);
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

.disconnect-btn-floating:hover { background: #e55; }
</style>
