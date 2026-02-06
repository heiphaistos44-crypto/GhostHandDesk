<template>
  <div class="settings-overlay" @click.self="handleClose">
    <div class="settings-panel">
      <!-- Header -->
      <div class="panel-header">
        <h2>⚙️ Paramètres</h2>
        <button @click="handleClose" class="close-btn">✕</button>
      </div>

      <!-- Content -->
      <div class="panel-content">
        <!-- Vidéo -->
        <section class="settings-section">
          <h3>🎥 Qualité vidéo</h3>

          <div class="setting-item">
            <label>Codec</label>
            <select v-model="settings.codec">
              <option value="H264">H.264 (Recommandé)</option>
              <option value="H265">H.265 / HEVC</option>
              <option value="VP8">VP8</option>
              <option value="VP9">VP9</option>
              <option value="JPEG">JPEG (Fallback)</option>
            </select>
          </div>

          <div class="setting-item">
            <label>Framerate</label>
            <div class="slider-container">
              <input
                type="range"
                v-model.number="settings.framerate"
                min="15"
                max="60"
                step="15"
              />
              <span class="slider-value">{{ settings.framerate }} FPS</span>
            </div>
          </div>

          <div class="setting-item">
            <label>Bitrate</label>
            <div class="slider-container">
              <input
                type="range"
                v-model.number="settings.bitrate"
                min="1000"
                max="10000"
                step="500"
              />
              <span class="slider-value">{{ settings.bitrate }} kbps</span>
            </div>
          </div>

          <div class="setting-item">
            <label>Qualité JPEG</label>
            <div class="slider-container">
              <input
                type="range"
                v-model.number="settings.quality"
                min="50"
                max="100"
                step="5"
              />
              <span class="slider-value">{{ settings.quality }}%</span>
            </div>
          </div>
        </section>

        <!-- Réseau -->
        <section class="settings-section">
          <h3>🌐 Réseau</h3>

          <div class="setting-item">
            <label>Serveur de signalement</label>
            <input
              type="text"
              v-model="settings.serverUrl"
              placeholder="ws://localhost:9000/ws"
            />
          </div>

          <div class="setting-item">
            <label>Serveurs STUN</label>
            <textarea
              v-model="stunServersText"
              rows="3"
              placeholder="Un serveur par ligne"
            ></textarea>
            <small>Serveurs STUN pour NAT traversal</small>
          </div>
        </section>

        <!-- Performance -->
        <section class="settings-section">
          <h3>⚡ Performance</h3>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.hardwareAcceleration" />
              <span>Accélération matérielle (NVENC/QSV)</span>
            </label>
          </div>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.lowLatencyMode" />
              <span>Mode faible latence</span>
            </label>
          </div>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.adaptiveBitrate" />
              <span>Bitrate adaptatif</span>
            </label>
          </div>
        </section>

        <!-- Interface -->
        <section class="settings-section">
          <h3>🎨 Interface</h3>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.showFpsCounter" />
              <span>Afficher le compteur FPS</span>
            </label>
          </div>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.showLatency" />
              <span>Afficher la latence</span>
            </label>
          </div>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.clipboardSync" />
              <span>Synchroniser le presse-papiers</span>
            </label>
          </div>
        </section>

        <!-- Sécurité -->
        <section class="settings-section">
          <h3>🔐 Sécurité</h3>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.requirePassword" />
              <span>Exiger un mot de passe pour les connexions</span>
            </label>
          </div>

          <div class="setting-item" v-if="settings.requirePassword">
            <label>Mot de passe de connexion</label>
            <input
              type="password"
              v-model="settings.connectionPassword"
              placeholder="••••••••"
            />
          </div>

          <div class="setting-item checkbox-item">
            <label>
              <input type="checkbox" v-model="settings.encryptData" />
              <span>Chiffrer les données (AES-256-GCM)</span>
            </label>
          </div>
        </section>
      </div>

      <!-- Footer -->
      <div class="panel-footer">
        <button @click="resetToDefaults" class="btn-secondary">
          Réinitialiser
        </button>
        <div class="footer-right">
          <button @click="handleClose" class="btn-secondary">
            Annuler
          </button>
          <button @click="handleSave" class="btn-primary">
            Sauvegarder
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

// Emits
const emit = defineEmits<{
  close: [];
  update: [settings: any];
}>();

// Settings state
const settings = ref({
  // Vidéo
  codec: 'H264',
  framerate: 30,
  bitrate: 4000,
  quality: 80,

  // Réseau
  serverUrl: 'ws://localhost:9000/ws',
  stunServers: [
    'stun:stun.l.google.com:19302',
    'stun:stun1.l.google.com:19302',
  ],

  // Performance
  hardwareAcceleration: false,
  lowLatencyMode: true,
  adaptiveBitrate: true,

  // Interface
  showFpsCounter: true,
  showLatency: true,
  clipboardSync: false,

  // Sécurité
  requirePassword: false,
  connectionPassword: '',
  encryptData: true,
});

// Computed pour STUN servers (textarea)
const stunServersText = computed({
  get: () => settings.value.stunServers.join('\n'),
  set: (value: string) => {
    settings.value.stunServers = value
      .split('\n')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
  },
});

// Méthodes
function handleClose() {
  emit('close');
}

function handleSave() {
  emit('update', { ...settings.value });
  emit('close');
  console.log('Paramètres sauvegardés:', settings.value);
}

function resetToDefaults() {
  if (confirm('Voulez-vous vraiment réinitialiser tous les paramètres ?')) {
    settings.value = {
      codec: 'H264',
      framerate: 30,
      bitrate: 4000,
      quality: 80,
      serverUrl: 'ws://localhost:9000/ws',
      stunServers: [
        'stun:stun.l.google.com:19302',
        'stun:stun1.l.google.com:19302',
      ],
      hardwareAcceleration: false,
      lowLatencyMode: true,
      adaptiveBitrate: true,
      showFpsCounter: true,
      showLatency: true,
      clipboardSync: false,
      requirePassword: false,
      connectionPassword: '',
      encryptData: true,
    };
  }
}
</script>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  padding: 20px;
}

.settings-panel {
  width: 100%;
  max-width: 700px;
  max-height: 90vh;
  background: #252526;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
}

/* Header */
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid #3e3e42;
}

.panel-header h2 {
  font-size: 20px;
  margin: 0;
}

.close-btn {
  background: transparent;
  border: none;
  color: #999;
  font-size: 24px;
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.2s;
}

.close-btn:hover {
  background: #3e3e42;
  color: #fff;
}

/* Content */
.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.settings-section {
  margin-bottom: 30px;
}

.settings-section:last-child {
  margin-bottom: 0;
}

.settings-section h3 {
  font-size: 15px;
  color: #4ec9b0;
  margin-bottom: 16px;
  padding-bottom: 8px;
  border-bottom: 1px solid #3e3e42;
}

.setting-item {
  margin-bottom: 20px;
}

.setting-item:last-child {
  margin-bottom: 0;
}

.setting-item label {
  display: block;
  font-size: 13px;
  color: #ccc;
  margin-bottom: 8px;
  font-weight: 500;
}

.setting-item input[type="text"],
.setting-item input[type="password"],
.setting-item select,
.setting-item textarea {
  width: 100%;
  padding: 10px 12px;
  background: #3c3c3c;
  border: 1px solid #555;
  border-radius: 6px;
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  transition: border-color 0.2s;
}

.setting-item input:focus,
.setting-item select:focus,
.setting-item textarea:focus {
  outline: none;
  border-color: #0e639c;
}

.setting-item textarea {
  resize: vertical;
  font-family: 'Courier New', monospace;
}

.setting-item small {
  display: block;
  margin-top: 6px;
  font-size: 11px;
  color: #888;
}

/* Checkbox */
.checkbox-item label {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  padding: 10px;
  background: #2d2d30;
  border-radius: 6px;
  transition: background 0.2s;
}

.checkbox-item label:hover {
  background: #3e3e42;
}

.checkbox-item input[type="checkbox"] {
  width: auto;
  margin: 0;
  cursor: pointer;
  accent-color: #0e639c;
}

/* Slider */
.slider-container {
  display: flex;
  align-items: center;
  gap: 15px;
}

.slider-container input[type="range"] {
  flex: 1;
  height: 6px;
  background: #3c3c3c;
  border-radius: 3px;
  outline: none;
  -webkit-appearance: none;
}

.slider-container input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  background: #0e639c;
  border-radius: 50%;
  cursor: pointer;
}

.slider-container input[type="range"]::-moz-range-thumb {
  width: 16px;
  height: 16px;
  background: #0e639c;
  border-radius: 50%;
  cursor: pointer;
  border: none;
}

.slider-value {
  min-width: 70px;
  text-align: right;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #4ec9b0;
}

/* Footer */
.panel-footer {
  display: flex;
  justify-content: space-between;
  padding: 16px 24px;
  border-top: 1px solid #3e3e42;
}

.footer-right {
  display: flex;
  gap: 10px;
}

.btn-primary,
.btn-secondary {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: #0e639c;
  color: #fff;
}

.btn-primary:hover {
  background: #1177bb;
}

.btn-secondary {
  background: #3c3c3c;
  color: #ccc;
}

.btn-secondary:hover {
  background: #4a4a4a;
}

/* Scrollbar */
.panel-content::-webkit-scrollbar {
  width: 8px;
}

.panel-content::-webkit-scrollbar-track {
  background: #1e1e1e;
}

.panel-content::-webkit-scrollbar-thumb {
  background: #555;
  border-radius: 4px;
}

.panel-content::-webkit-scrollbar-thumb:hover {
  background: #666;
}
</style>
