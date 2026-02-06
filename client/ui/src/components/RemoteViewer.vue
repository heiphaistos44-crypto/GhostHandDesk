<template>
  <div class="remote-viewer">
    <!-- Toolbar -->
    <div class="viewer-toolbar">
      <div class="toolbar-left">
        <button @click="handleDisconnect" class="toolbar-btn disconnect-btn" title="Déconnecter">
          <span>🔌</span>
          <span class="btn-label">Déconnecter</span>
        </button>
      </div>

      <div class="toolbar-center">
        <span class="connection-info">
          Connecté à <code>{{ connectionId }}</code>
        </span>
        <div class="stats">
          <span class="stat">FPS: {{ fps }}</span>
          <span class="stat">Latence: {{ latency }}ms</span>
        </div>
      </div>

      <div class="toolbar-right">
        <button @click="toggleFullscreen" class="toolbar-btn" title="Plein écran">
          <span>{{ isFullscreen ? '🗗' : '🗖' }}</span>
        </button>
        <button @click="captureScreenshot" class="toolbar-btn" title="Capture d'écran">
          <span>📷</span>
        </button>
        <button @click="showQuality = !showQuality" class="toolbar-btn" title="Qualité">
          <span>⚙️</span>
        </button>
      </div>
    </div>

    <!-- Quality Dropdown -->
    <div v-if="showQuality" class="quality-dropdown">
      <div class="dropdown-header">
        <h4>Qualité du streaming</h4>
        <button @click="showQuality = false" class="close-btn">✕</button>
      </div>
      <div class="quality-options">
        <label>
          <input type="radio" value="low" v-model="quality" @change="updateQuality" />
          <span>Basse (15 FPS, économie bande passante)</span>
        </label>
        <label>
          <input type="radio" value="medium" v-model="quality" @change="updateQuality" />
          <span>Moyenne (30 FPS, équilibré)</span>
        </label>
        <label>
          <input type="radio" value="high" v-model="quality" @change="updateQuality" />
          <span>Haute (60 FPS, haute qualité)</span>
        </label>
      </div>
    </div>

    <!-- Canvas pour le streaming -->
    <div class="canvas-container" ref="containerRef">
      <canvas
        ref="canvasRef"
        class="stream-canvas"
        @mousedown="handleMouseDown"
        @mouseup="handleMouseUp"
        @mousemove="handleMouseMove"
        @wheel="handleWheel"
        @contextmenu.prevent
        tabindex="0"
        @keydown="handleKeyDown"
        @keyup="handleKeyUp"
      />

      <!-- Overlay de connexion -->
      <div v-if="!streaming" class="overlay">
        <div class="overlay-content">
          <div class="spinner-large"></div>
          <p>Attente du streaming vidéo...</p>
          <small>La connexion WebRTC est en cours d'établissement</small>
        </div>
      </div>

      <!-- Indicateur FPS -->
      <div class="fps-indicator" :class="{ 'fps-low': fps < 20 }">
        {{ fps }} FPS
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Props
interface Props {
  connectionId: string;
}

const props = defineProps<Props>();

// Emits
const emit = defineEmits<{
  disconnect: [];
}>();

// Refs
const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);

// État
const streaming = ref(false);
const fps = ref(0);
const latency = ref(0);
const quality = ref('medium');
const showQuality = ref(false);
const isFullscreen = ref(false);

// Variables de performance
let frameCount = 0;
let lastFpsUpdate = Date.now();
let unlistenVideo: UnlistenFn | null = null;

// Lifecycle
onMounted(async () => {
  console.log('RemoteViewer monté, connexion:', props.connectionId);

  // Focus sur le canvas pour les événements clavier
  canvasRef.value?.focus();

  // Écouter les frames vidéo
  try {
    unlistenVideo = await listen<VideoFramePayload>('video-frame', (event) => {
      handleVideoFrame(event.payload);
    });
    console.log('Listener vidéo configuré');
  } catch (error) {
    console.error('Erreur configuration listener:', error);
  }

  // Calculer FPS
  setInterval(updateFps, 1000);
});

onUnmounted(() => {
  // Nettoyer les listeners
  if (unlistenVideo) {
    unlistenVideo();
  }
});

// Types
interface VideoFramePayload {
  data: number[]; // Uint8Array converti en array
  width: number;
  height: number;
  timestamp: number;
}

// Constantes de sécurité pour la validation des frames
const MAX_FRAME_WIDTH = 3840;  // 4K
const MAX_FRAME_HEIGHT = 2160; // 4K
const MAX_FRAME_DATA_SIZE = 10 * 1024 * 1024; // 10 MB

// Méthodes
function handleVideoFrame(payload: VideoFramePayload) {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  // SÉCURITÉ : Valider les dimensions de la frame
  if (!payload.width || !payload.height ||
      payload.width <= 0 || payload.height <= 0 ||
      payload.width > MAX_FRAME_WIDTH || payload.height > MAX_FRAME_HEIGHT) {
    console.error(
      `[SÉCURITÉ] Dimensions de frame invalides ou dangereuses: ${payload.width}x${payload.height}. ` +
      `Limites: ${MAX_FRAME_WIDTH}x${MAX_FRAME_HEIGHT}`
    );
    return;
  }

  // SÉCURITÉ : Valider la taille des données
  if (!payload.data || payload.data.length === 0 || payload.data.length > MAX_FRAME_DATA_SIZE) {
    console.error(
      `[SÉCURITÉ] Taille de données invalide: ${payload.data?.length || 0} bytes. ` +
      `Limite: ${MAX_FRAME_DATA_SIZE} bytes`
    );
    return;
  }

  // SÉCURITÉ : Vérifier que les données sont des nombres valides
  if (!Array.isArray(payload.data)) {
    console.error('[SÉCURITÉ] Format de données invalide: attendu Array');
    return;
  }

  // Ajuster taille canvas si nécessaire (dimensions déjà validées)
  if (canvas.width !== payload.width || canvas.height !== payload.height) {
    canvas.width = payload.width;
    canvas.height = payload.height;
  }

  // Décoder et dessiner selon le format
  try {
    // Les données sont encodées en JPEG - créer un Blob et une Image
    const blob = new Blob([new Uint8Array(payload.data)], { type: 'image/jpeg' });
    const url = URL.createObjectURL(blob);

    const img = new Image();
    img.onload = () => {
      // SÉCURITÉ : Vérifier que l'image décodée a les bonnes dimensions
      if (img.width !== payload.width || img.height !== payload.height) {
        console.warn(
          `[SÉCURITÉ] Dimensions image décodée différentes: ` +
          `attendu ${payload.width}x${payload.height}, ` +
          `obtenu ${img.width}x${img.height}`
        );
      }

      // Dessiner l'image sur le canvas (dimensions validées)
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

      // Libérer la mémoire
      URL.revokeObjectURL(url);

      // Marquer comme streaming
      if (!streaming.value) {
        streaming.value = true;
      }

      // Compter frame
      frameCount++;

      // Calculer latence
      const now = Date.now();
      latency.value = Math.max(0, now - payload.timestamp);
    };

    img.onerror = (err) => {
      console.error('[SÉCURITÉ] Erreur chargement image (format invalide ou corrompu):', err);
      URL.revokeObjectURL(url);
    };

    img.src = url;
  } catch (error) {
    console.error('[SÉCURITÉ] Erreur traitement frame:', error);
  }
}

function updateFps() {
  const now = Date.now();
  const elapsed = (now - lastFpsUpdate) / 1000;

  fps.value = Math.round(frameCount / elapsed);

  frameCount = 0;
  lastFpsUpdate = now;
}

// Gestion événements souris
async function handleMouseDown(event: MouseEvent) {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const x = Math.round((event.clientX - rect.left) * (canvas.width / rect.width));
  const y = Math.round((event.clientY - rect.top) * (canvas.height / rect.height));

  try {
    await invoke('send_mouse_event', {
      event: {
        x,
        y,
        button: event.button === 0 ? 'left' : event.button === 2 ? 'right' : 'middle',
        type: 'press',
      },
    });
  } catch (error) {
    console.error('Erreur envoi mouse down:', error);
  }
}

async function handleMouseUp(event: MouseEvent) {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const x = Math.round((event.clientX - rect.left) * (canvas.width / rect.width));
  const y = Math.round((event.clientY - rect.top) * (canvas.height / rect.height));

  try {
    await invoke('send_mouse_event', {
      event: {
        x,
        y,
        button: event.button === 0 ? 'left' : event.button === 2 ? 'right' : 'middle',
        type: 'release',
      },
    });
  } catch (error) {
    console.error('Erreur envoi mouse up:', error);
  }
}

async function handleMouseMove(event: MouseEvent) {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const x = Math.round((event.clientX - rect.left) * (canvas.width / rect.width));
  const y = Math.round((event.clientY - rect.top) * (canvas.height / rect.height));

  try {
    await invoke('send_mouse_event', {
      event: {
        x,
        y,
        button: 'none',
        type: 'move',
      },
    });
  } catch (error) {
    // Ne pas logger les erreurs de mouvement (trop fréquent)
  }
}

async function handleWheel(event: WheelEvent) {
  event.preventDefault();

  try {
    await invoke('send_mouse_event', {
      event: {
        x: 0,
        y: 0,
        button: 'none',
        type: 'scroll',
        delta: event.deltaY,
      },
    });
  } catch (error) {
    console.error('Erreur envoi scroll:', error);
  }
}

// Gestion événements clavier
async function handleKeyDown(event: KeyboardEvent) {
  event.preventDefault();

  try {
    await invoke('send_keyboard_event', {
      event: {
        key: event.key,
        code: event.code,
        type: 'press',
        modifiers: {
          ctrl: event.ctrlKey,
          shift: event.shiftKey,
          alt: event.altKey,
          meta: event.metaKey,
        },
      },
    });
  } catch (error) {
    console.error('Erreur envoi key down:', error);
  }
}

async function handleKeyUp(event: KeyboardEvent) {
  event.preventDefault();

  try {
    await invoke('send_keyboard_event', {
      event: {
        key: event.key,
        code: event.code,
        type: 'release',
        modifiers: {
          ctrl: event.ctrlKey,
          shift: event.shiftKey,
          alt: event.altKey,
          meta: event.metaKey,
        },
      },
    });
  } catch (error) {
    console.error('Erreur envoi key up:', error);
  }
}

// Actions
function handleDisconnect() {
  if (confirm('Voulez-vous vraiment vous déconnecter ?')) {
    emit('disconnect');
  }
}

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    containerRef.value?.requestFullscreen();
    isFullscreen.value = true;
  } else {
    document.exitFullscreen();
    isFullscreen.value = false;
  }
}

function captureScreenshot() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  // Créer un lien de téléchargement
  canvas.toBlob((blob) => {
    if (!blob) return;

    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `screenshot-${Date.now()}.png`;
    a.click();
    URL.revokeObjectURL(url);
  });
}

function updateQuality() {
  console.log('Qualité mise à jour:', quality.value);
  // TODO: Envoyer au backend pour ajuster le framerate/bitrate
}
</script>

<style scoped>
.remote-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #000;
}

/* Toolbar */
.viewer-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: rgba(45, 45, 48, 0.95);
  border-bottom: 1px solid #3e3e42;
  height: 50px;
  z-index: 10;
}

.toolbar-left,
.toolbar-center,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-center {
  flex: 1;
  justify-content: center;
  flex-direction: column;
}

.connection-info {
  font-size: 13px;
  color: #9d9d9d;
}

.connection-info code {
  color: #4ec9b0;
  font-family: monospace;
}

.stats {
  display: flex;
  gap: 15px;
  font-size: 11px;
  color: #666;
}

.stat {
  padding: 2px 6px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
}

.toolbar-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: transparent;
  border: 1px solid #3e3e42;
  border-radius: 4px;
  color: #ccc;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.toolbar-btn:hover {
  background: #3e3e42;
  border-color: #555;
}

.disconnect-btn {
  border-color: #c44;
  color: #f88;
}

.disconnect-btn:hover {
  background: #c44;
  color: #fff;
}

.btn-label {
  font-size: 12px;
}

/* Quality Dropdown */
.quality-dropdown {
  position: absolute;
  top: 60px;
  right: 20px;
  width: 320px;
  background: #252526;
  border: 1px solid #3e3e42;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
  z-index: 100;
  padding: 15px;
}

.dropdown-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.dropdown-header h4 {
  font-size: 14px;
  margin: 0;
}

.close-btn {
  background: transparent;
  border: none;
  color: #999;
  font-size: 18px;
  cursor: pointer;
  padding: 0;
}

.close-btn:hover {
  color: #fff;
}

.quality-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.quality-options label {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  background: #2d2d30;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.2s;
}

.quality-options label:hover {
  background: #3e3e42;
}

.quality-options input[type="radio"] {
  accent-color: #0e639c;
}

/* Canvas */
.canvas-container {
  flex: 1;
  position: relative;
  overflow: hidden;
  background: #000;
  display: flex;
  justify-content: center;
  align-items: center;
}

.stream-canvas {
  width: 100%;
  height: 100%;
  object-fit: contain;
  cursor: crosshair;
}

.stream-canvas:focus {
  outline: 2px solid #0e639c;
  outline-offset: -2px;
}

/* Overlay */
.overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 5;
}

.overlay-content {
  text-align: center;
  color: #ccc;
}

.spinner-large {
  width: 48px;
  height: 48px;
  border: 4px solid rgba(255, 255, 255, 0.1);
  border-top-color: #4ec9b0;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 0 auto 20px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.overlay-content p {
  font-size: 16px;
  margin-bottom: 8px;
}

.overlay-content small {
  font-size: 13px;
  color: #888;
}

/* FPS Indicator */
.fps-indicator {
  position: absolute;
  top: 10px;
  left: 10px;
  padding: 4px 10px;
  background: rgba(0, 0, 0, 0.7);
  color: #4ec9b0;
  font-size: 12px;
  font-family: monospace;
  border-radius: 4px;
  z-index: 10;
}

.fps-indicator.fps-low {
  color: #ffa500;
}
</style>
