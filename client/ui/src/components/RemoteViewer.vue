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

// Dimensions de l'écran distant et zone de dessin réelle dans le canvas
const remoteWidth = ref(0);
const remoteHeight = ref(0);
const drawRect = ref({ x: 0, y: 0, w: 0, h: 0 });

// Variables de performance
let frameCount = 0;
let lastFpsUpdate = Date.now();
let fpsIntervalId: ReturnType<typeof setInterval> | null = null;
let videoEventHandler: ((event: Event) => void) | null = null;
let resizeObserver: ResizeObserver | null = null;

// Lifecycle
onMounted(async () => {
  console.log('RemoteViewer monté, connexion:', props.connectionId);

  // Focus sur le canvas pour les événements clavier
  canvasRef.value?.focus();

  // Écouter les frames vidéo via DOM CustomEvent
  // (window.eval() + CustomEvent car le Tauri event system ne fonctionne pas)
  videoEventHandler = ((event: Event) => {
    const detail = (event as CustomEvent).detail;
    // Décoder base64 en Uint8Array
    const binaryString = atob(detail.data);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    handleVideoFrame({
      data: Array.from(bytes),
      width: detail.width,
      height: detail.height,
      timestamp: detail.timestamp,
    });
  });
  window.addEventListener('ghosthand-video-frame', videoEventHandler);
  console.log('Listener vidéo configuré (CustomEvent)');

  // Observer les changements de taille du container
  if (containerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      recalcDrawRect();
    });
    resizeObserver.observe(containerRef.value);
  }

  // Calculer FPS
  fpsIntervalId = setInterval(updateFps, 1000);
});

onUnmounted(() => {
  if (videoEventHandler) {
    window.removeEventListener('ghosthand-video-frame', videoEventHandler);
  }
  if (fpsIntervalId) {
    clearInterval(fpsIntervalId);
  }
  if (resizeObserver) {
    resizeObserver.disconnect();
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

// Recalculer la zone de dessin quand le container change de taille
function recalcDrawRect() {
  const canvas = canvasRef.value;
  const container = containerRef.value;
  if (!canvas || !container || !remoteWidth.value || !remoteHeight.value) return;

  // Canvas interne = taille du container (pixels CSS * devicePixelRatio pour netteté)
  const cw = container.clientWidth;
  const ch = container.clientHeight;
  if (canvas.width !== cw || canvas.height !== ch) {
    canvas.width = cw;
    canvas.height = ch;
  }

  // Calculer le rect de dessin en respectant le ratio d'aspect
  const imgAspect = remoteWidth.value / remoteHeight.value;
  const containerAspect = cw / ch;

  let dx: number, dy: number, dw: number, dh: number;
  if (imgAspect > containerAspect) {
    // Image plus large → fit sur la largeur, barres haut/bas
    dw = cw;
    dh = cw / imgAspect;
    dx = 0;
    dy = (ch - dh) / 2;
  } else {
    // Image plus haute → fit sur la hauteur, barres gauche/droite
    dh = ch;
    dw = ch * imgAspect;
    dx = (cw - dw) / 2;
    dy = 0;
  }

  drawRect.value = { x: dx, y: dy, w: dw, h: dh };
}

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
      `[SÉCURITÉ] Dimensions de frame invalides: ${payload.width}x${payload.height}`
    );
    return;
  }

  // SÉCURITÉ : Valider la taille des données
  if (!payload.data || payload.data.length === 0 || payload.data.length > MAX_FRAME_DATA_SIZE) {
    console.error(
      `[SÉCURITÉ] Taille de données invalide: ${payload.data?.length || 0} bytes`
    );
    return;
  }

  if (!Array.isArray(payload.data)) {
    console.error('[SÉCURITÉ] Format de données invalide: attendu Array');
    return;
  }

  // Mettre à jour les dimensions de l'écran distant
  if (remoteWidth.value !== payload.width || remoteHeight.value !== payload.height) {
    remoteWidth.value = payload.width;
    remoteHeight.value = payload.height;
    recalcDrawRect();
  }

  // S'assurer que le canvas est dimensionné au container
  const container = containerRef.value;
  if (container) {
    const cw = container.clientWidth;
    const ch = container.clientHeight;
    if (canvas.width !== cw || canvas.height !== ch) {
      canvas.width = cw;
      canvas.height = ch;
      recalcDrawRect();
    }
  }

  // Décoder et dessiner
  try {
    const blob = new Blob([new Uint8Array(payload.data)], { type: 'image/jpeg' });
    const url = URL.createObjectURL(blob);

    const img = new Image();
    img.onload = () => {
      const dr = drawRect.value;
      // Effacer le canvas (barres noires)
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      // Dessiner l'image dans la zone calculée (ratio préservé)
      ctx.drawImage(img, dr.x, dr.y, dr.w, dr.h);

      URL.revokeObjectURL(url);

      if (!streaming.value) {
        streaming.value = true;
      }

      frameCount++;

      const now = Date.now();
      latency.value = Math.max(0, now - payload.timestamp);
    };

    img.onerror = (err) => {
      console.error('[SÉCURITÉ] Erreur chargement image:', err);
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

// Convertir les coordonnées canvas CSS → coordonnées écran distant
function canvasToRemote(event: MouseEvent): { x: number; y: number } | null {
  const canvas = canvasRef.value;
  if (!canvas || !remoteWidth.value || !remoteHeight.value) return null;

  const rect = canvas.getBoundingClientRect();
  const dr = drawRect.value;

  // Position dans le canvas en pixels CSS
  const cssX = event.clientX - rect.left;
  const cssY = event.clientY - rect.top;

  // Ratio CSS → pixels internes du canvas
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;

  // Position dans le canvas en pixels internes
  const canvasX = cssX * scaleX;
  const canvasY = cssY * scaleY;

  // Position relative à la zone de dessin de l'image (0..1)
  const relX = (canvasX - dr.x) / dr.w;
  const relY = (canvasY - dr.y) / dr.h;

  // Clamper dans [0, 1] pour rester dans la zone image
  const clampedX = Math.max(0, Math.min(1, relX));
  const clampedY = Math.max(0, Math.min(1, relY));

  // Convertir en coordonnées de l'écran distant
  return {
    x: Math.round(clampedX * remoteWidth.value),
    y: Math.round(clampedY * remoteHeight.value),
  };
}

// Gestion événements souris
async function handleMouseDown(event: MouseEvent) {
  const coords = canvasToRemote(event);
  if (!coords) return;

  try {
    await invoke('send_mouse_event', {
      event: {
        x: coords.x,
        y: coords.y,
        button: event.button === 0 ? 'left' : event.button === 2 ? 'right' : 'middle',
        type: 'down',
      },
    });
  } catch (error) {
    console.error('Erreur envoi mouse down:', error);
  }
}

async function handleMouseUp(event: MouseEvent) {
  const coords = canvasToRemote(event);
  if (!coords) return;

  try {
    await invoke('send_mouse_event', {
      event: {
        x: coords.x,
        y: coords.y,
        button: event.button === 0 ? 'left' : event.button === 2 ? 'right' : 'middle',
        type: 'up',
      },
    });
  } catch (error) {
    console.error('Erreur envoi mouse up:', error);
  }
}

async function handleMouseMove(event: MouseEvent) {
  const coords = canvasToRemote(event);
  if (!coords) return;

  try {
    await invoke('send_mouse_event', {
      event: {
        x: coords.x,
        y: coords.y,
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
        delta: Math.round(event.deltaY),
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
        type: 'keydown',
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
        type: 'keyup',
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

async function updateQuality() {
  // Presets qualité : low / medium / high
  const presets: Record<string, { framerate: number; bitrate: number; codec: string }> = {
    low: { framerate: 15, bitrate: 1500, codec: 'JPEG' },
    medium: { framerate: 30, bitrate: 4000, codec: 'JPEG' },
    high: { framerate: 60, bitrate: 8000, codec: 'H264' },
  };

  const preset = presets[quality.value] || presets.medium;

  try {
    const currentConfig = await invoke<any>('get_config');
    currentConfig.video_config.framerate = preset.framerate;
    currentConfig.video_config.bitrate = preset.bitrate;
    currentConfig.video_config.codec = preset.codec;

    await invoke('update_config', { newConfig: currentConfig });
    console.log(`[VIEWER] Qualité mise à jour: ${quality.value}`, preset);
  } catch (error) {
    console.error('Erreur mise à jour qualité:', error);
  }
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
  cursor: default;
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
