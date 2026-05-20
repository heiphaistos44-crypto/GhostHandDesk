<template>
  <div class="connect-dialog">
    <div class="dialog-container">
      <div class="dialog-header">
        <h2>Se connecter à un appareil distant</h2>
        <p class="subtitle">Entrez le Device ID de l'appareil que vous souhaitez contrôler</p>
      </div>

      <!-- Appareils détectés sur le réseau local -->
      <div v-if="discoveredPeers.length > 0" class="discovered-section">
        <h3>Appareils détectés sur le réseau</h3>
        <div class="discovered-list">
          <button
            v-for="peer in discoveredPeers"
            :key="peer.device_id"
            class="discovered-peer"
            @click="connectToPeer(peer)"
            :disabled="connecting"
          >
            <span class="peer-icon">🖥️</span>
            <div class="peer-info">
              <span class="peer-id">{{ peer.device_id }}</span>
              <span class="peer-ip">{{ peer.ip }}:{{ peer.port }}</span>
            </div>
            <span class="peer-arrow">→</span>
          </button>
        </div>
      </div>

      <form @submit.prevent="handleSubmit" class="connect-form">
        <!-- Server URL (collapsible) -->
        <div class="form-group server-group">
          <div class="server-header" @click="showServerUrl = !showServerUrl">
            <label>Serveur de signalement</label>
            <span class="server-toggle">{{ showServerUrl ? '▲' : '▼' }}</span>
          </div>
          <div v-if="showServerUrl" class="server-content">
            <div class="server-current">
              <span class="server-status" :class="serverConnected ? 'server-ok' : 'server-err'"></span>
              <span class="server-url-label">{{ currentServerUrl || 'Non configuré' }}</span>
            </div>
            <input
              v-model="serverUrl"
              type="text"
              placeholder="ws://192.168.1.x:9000/ws"
              class="form-input server-input"
              :disabled="connecting || changingServer"
            />
            <div class="server-actions">
              <button
                type="button"
                class="server-apply-btn"
                :disabled="!serverUrl || serverUrl === currentServerUrl || connecting || changingServer"
                @click="handleChangeServer"
              >
                <span v-if="!changingServer">Appliquer</span>
                <span v-else class="connecting-text">
                  <span class="spinner small"></span>
                  Reconnexion...
                </span>
              </button>
              <button
                type="button"
                class="server-reset-btn"
                @click="serverUrl = 'ws://localhost:9000/ws'"
                :disabled="connecting || changingServer"
              >
                Reset
              </button>
            </div>
            <span class="input-hint">Pour contrôler un PC distant, entrez l'IP de la machine qui héberge le serveur (visible dans son header)</span>
          </div>
        </div>

        <!-- Device ID Input -->
        <div class="form-group">
          <label for="target-id">Device ID de la cible</label>
          <input
            id="target-id"
            v-model="targetId"
            type="text"
            placeholder="GHD-abc123def456"
            :disabled="connecting"
            required
            class="form-input"
            autocomplete="off"
          />
          <span class="input-hint">Format: GHD-xxxxxxxxxxxxx</span>
        </div>

        <!-- Password Input (optionnel) -->
        <div class="form-group">
          <label for="password">Mot de passe (optionnel)</label>
          <input
            id="password"
            v-model="password"
            type="password"
            placeholder="••••••••"
            :disabled="connecting"
            class="form-input"
            autocomplete="off"
          />
          <span class="input-hint">Laissez vide si aucun mot de passe n'est requis</span>
        </div>

        <!-- Error message -->
        <div v-if="error || serverError" class="error-message">
          <span class="error-icon">⚠️</span>
          <span>{{ error || serverError }}</span>
        </div>

        <!-- Connect / Cancel Buttons -->
        <div v-if="!connecting" class="connect-actions">
          <button
            type="submit"
            class="connect-btn"
            :disabled="!targetId"
          >
            Se connecter
          </button>
        </div>
        <div v-else class="connect-actions connecting-actions">
          <button type="button" class="connect-btn connecting" disabled>
            <span class="connecting-text">
              <span class="spinner"></span>
              Connexion en cours...
            </span>
          </button>
          <button type="button" class="cancel-btn" @click="emit('cancel')">
            Annuler
          </button>
        </div>

        <!-- Help text -->
        <div class="help-text">
          <p>💡 <strong>Astuce :</strong> L'appareil distant doit être en ligne et afficher son Device ID.</p>
        </div>
      </form>

      <!-- Quick Actions -->
      <div class="quick-actions">
        <h3>Actions rapides</h3>
        <div class="actions-grid">
          <button class="action-card" @click="showHelp">
            <span class="action-icon">❓</span>
            <span class="action-label">Aide</span>
          </button>
          <button class="action-card" @click="openSettings">
            <span class="action-icon">⚙️</span>
            <span class="action-label">Paramètres</span>
          </button>
          <button class="action-card" @click="showAbout">
            <span class="action-icon">ℹ️</span>
            <span class="action-label">À propos</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// Props
interface Props {
  connecting?: boolean;
  error?: string;
  initialServerUrl?: string;
}

const props = withDefaults(defineProps<Props>(), {
  connecting: false,
  error: '',
  initialServerUrl: '',
});

// Emits
const emit = defineEmits<{
  connect: [targetId: string, password: string | null];
  serverChanged: [serverUrl: string];
  cancel: [];
}>();

// État local
const targetId = ref('');
const password = ref('');
const showServerUrl = ref(false);
const serverUrl = ref('');
const currentServerUrl = ref('');
const serverConnected = ref(true);
const changingServer = ref(false);
const serverError = ref('');

// Auto-discovery LAN
interface DiscoveredPeer {
  device_id: string;
  ip: string;
  port: number;
  last_seen: number;
}
const discoveredPeers = ref<DiscoveredPeer[]>([]);
let discoveryInterval: ReturnType<typeof setInterval> | null = null;

// Charger l'URL du serveur actuel au montage
onMounted(async () => {
  // Polling des peers découverts toutes les 2 secondes
  discoveryInterval = setInterval(async () => {
    try {
      discoveredPeers.value = await invoke<DiscoveredPeer[]>('get_discovered_peers');
    } catch (e) {
      // Silencieux
    }
  }, 2000);
  // Premier fetch immédiat
  try {
    discoveredPeers.value = await invoke<DiscoveredPeer[]>('get_discovered_peers');
  } catch (e) {}

  try {
    const info = await invoke<any>('get_network_info');
    currentServerUrl.value = info.server_url || 'ws://localhost:9000/ws';
    serverUrl.value = currentServerUrl.value;
  } catch (e) {
    console.error('Erreur chargement info serveur:', e);
    currentServerUrl.value = 'ws://localhost:9000/ws';
    serverUrl.value = currentServerUrl.value;
  }
});

// Méthodes
async function handleSubmit() {
  if (!targetId.value.trim()) return;

  // Auto-détection : chercher si le Device ID existe dans les peers découverts
  const target = targetId.value.trim();
  const matchedPeer = discoveredPeers.value.find(p => p.device_id === target);

  if (matchedPeer) {
    // Peer trouvé sur le LAN → auto-configurer le serveur si nécessaire
    await connectToPeer(matchedPeer);
  } else {
    // Peer non trouvé → utiliser le serveur actuel (localhost ou configuré)
    emit('connect', target, password.value.trim() || null);
  }
}

async function handleChangeServer() {
  if (!serverUrl.value.trim()) return;

  changingServer.value = true;
  serverError.value = '';

  try {
    // Mettre à jour l'URL du serveur et réinitialiser la session
    await invoke('update_server_url', { serverUrl: serverUrl.value.trim() });

    currentServerUrl.value = serverUrl.value.trim();
    serverConnected.value = true;
    emit('serverChanged', currentServerUrl.value);
    console.log('[ConnectDialog] Serveur changé:', currentServerUrl.value);
  } catch (e: any) {
    console.error('Erreur changement serveur:', e);
    serverError.value = e.message || e || 'Impossible de se connecter au serveur';
    serverConnected.value = false;
  } finally {
    changingServer.value = false;
  }
}

onUnmounted(() => {
  if (discoveryInterval) {
    clearInterval(discoveryInterval);
  }
});

async function connectToPeer(peer: DiscoveredPeer) {
  // Vérifier si le peer est sur la même machine (même IP locale)
  let localIp = '';
  try {
    const info = await invoke<any>('get_network_info');
    localIp = info.local_ip || '';
  } catch (e) {}

  const isSameMachine = peer.ip === localIp || peer.ip === '127.0.0.1';

  // Changer le serveur URL seulement si le peer est sur une AUTRE machine
  if (!isSameMachine) {
    const peerServerUrl = `ws://${peer.ip}:${peer.port}/ws`;
    if (peerServerUrl !== currentServerUrl.value) {
      changingServer.value = true;
      serverError.value = '';
      try {
        await invoke('update_server_url', { serverUrl: peerServerUrl });
        currentServerUrl.value = peerServerUrl;
        serverUrl.value = peerServerUrl;
        serverConnected.value = true;
        emit('serverChanged', peerServerUrl);
      } catch (e: any) {
        serverError.value = e.message || e || 'Impossible de se connecter';
        serverConnected.value = false;
        changingServer.value = false;
        return;
      }
      changingServer.value = false;
    }
  }

  // Lancer la connexion vers le device
  emit('connect', peer.device_id, null);
}

function showHelp() {
  alert('Documentation complète disponible dans README.md');
}

function openSettings() {
  console.log('Ouvrir les paramètres');
}

function showAbout() {
  alert('GhostHandDesk v0.1.0\nBureau à distance open-source\n\nMade with ❤️ and Rust 🦀');
}
</script>

<style scoped>
.connect-dialog {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  padding: 20px;
  background: linear-gradient(135deg, #1e1e1e 0%, #2d2d30 100%);
}

.dialog-container {
  max-width: 500px;
  width: 100%;
  background: #252526;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  padding: 30px;
}

.dialog-header {
  margin-bottom: 30px;
  text-align: center;
}

.dialog-header h2 {
  font-size: 24px;
  margin-bottom: 8px;
  color: #fff;
}

.subtitle {
  font-size: 14px;
  color: #9d9d9d;
  line-height: 1.5;
}

/* Form */
.connect-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-group label {
  font-size: 13px;
  font-weight: 500;
  color: #ccc;
}

.form-input {
  padding: 12px 16px;
  background: #3c3c3c;
  border: 1px solid #555;
  border-radius: 6px;
  color: #fff;
  font-size: 14px;
  font-family: 'Courier New', monospace;
  transition: border-color 0.2s, background 0.2s;
}

.form-input:focus {
  outline: none;
  border-color: #0e639c;
  background: #454545;
}

.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.input-hint {
  font-size: 12px;
  color: #888;
}

/* Error */
.error-message {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid rgba(244, 67, 54, 0.3);
  border-radius: 6px;
  color: #ff6b6b;
  font-size: 13px;
}

.error-icon {
  font-size: 18px;
}

/* Button */
.connect-btn {
  padding: 14px;
  background: #0e639c;
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s, transform 0.1s;
}

.connect-btn:hover:not(:disabled) {
  background: #1177bb;
  transform: translateY(-1px);
}

.connect-btn:active:not(:disabled) {
  transform: translateY(0);
}

.connect-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.connect-actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.connecting-actions {
  gap: 10px;
}

.connect-btn.connecting {
  opacity: 0.7;
}

.cancel-btn {
  padding: 12px;
  background: transparent;
  border: 1px solid #c44;
  border-radius: 6px;
  color: #f88;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.cancel-btn:hover {
  background: #c44;
  color: #fff;
}

.connecting-text {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Help */
.help-text {
  padding: 12px;
  background: rgba(78, 201, 176, 0.1);
  border-radius: 6px;
  font-size: 13px;
  color: #9d9d9d;
}

/* Discovered peers */
.discovered-section {
  margin-bottom: 20px;
}

.discovered-section h3 {
  font-size: 13px;
  color: #4ec9b0;
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.discovered-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.discovered-peer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #2d2d30;
  border: 1px solid #3e3e42;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  color: #ccc;
  text-align: left;
  width: 100%;
}

.discovered-peer:hover:not(:disabled) {
  background: #3e3e42;
  border-color: #4ec9b0;
  transform: translateY(-1px);
}

.discovered-peer:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.peer-icon {
  font-size: 24px;
}

.peer-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.peer-id {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #4ec9b0;
}

.peer-ip {
  font-size: 11px;
  color: #888;
}

.peer-arrow {
  font-size: 18px;
  color: #4ec9b0;
}

/* Server URL section */
.server-group {
  background: #2d2d30;
  border-radius: 8px;
  padding: 12px;
  border: 1px solid #3e3e42;
}

.server-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
}

.server-header label {
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  color: #ccc;
}

.server-toggle {
  font-size: 10px;
  color: #888;
}

.server-content {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.server-current {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #999;
}

.server-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.server-status.server-ok {
  background: #4ec9b0;
}

.server-status.server-err {
  background: #f44;
}

.server-url-label {
  font-family: 'Courier New', monospace;
  font-size: 12px;
}

.server-input {
  font-size: 13px !important;
  padding: 8px 12px !important;
}

.server-actions {
  display: flex;
  gap: 8px;
}

.server-apply-btn {
  flex: 1;
  padding: 8px 12px;
  background: #0e639c;
  border: none;
  border-radius: 4px;
  color: #fff;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
}

.server-apply-btn:hover:not(:disabled) {
  background: #1177bb;
}

.server-apply-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.server-reset-btn {
  padding: 8px 12px;
  background: #3c3c3c;
  border: 1px solid #555;
  border-radius: 4px;
  color: #ccc;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
}

.server-reset-btn:hover:not(:disabled) {
  background: #4c4c4c;
}

.server-reset-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinner.small {
  width: 12px;
  height: 12px;
}

/* Quick Actions */
.quick-actions {
  margin-top: 30px;
  padding-top: 30px;
  border-top: 1px solid #3e3e42;
}

.quick-actions h3 {
  font-size: 14px;
  color: #9d9d9d;
  margin-bottom: 15px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.action-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 12px;
  background: #2d2d30;
  border: 1px solid #3e3e42;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.action-card:hover {
  background: #3e3e42;
  border-color: #555;
  transform: translateY(-2px);
}

.action-icon {
  font-size: 24px;
}

.action-label {
  font-size: 12px;
  color: #ccc;
}
</style>
