<template>
  <div class="connect-dialog">
    <div class="dialog-container">
      <div class="dialog-header">
        <h2>Se connecter à un appareil distant</h2>
        <p class="subtitle">Entrez le Device ID de l'appareil que vous souhaitez contrôler</p>
      </div>

      <form @submit.prevent="handleSubmit" class="connect-form">
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
        <div v-if="error" class="error-message">
          <span class="error-icon">⚠️</span>
          <span>{{ error }}</span>
        </div>

        <!-- Connect Button -->
        <button
          type="submit"
          class="connect-btn"
          :disabled="!targetId || connecting"
        >
          <span v-if="!connecting">Se connecter</span>
          <span v-else class="connecting-text">
            <span class="spinner"></span>
            Connexion en cours...
          </span>
        </button>

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
import { ref } from 'vue';

// Props
interface Props {
  connecting?: boolean;
  error?: string;
}

const props = withDefaults(defineProps<Props>(), {
  connecting: false,
  error: '',
});

// Emits
const emit = defineEmits<{
  connect: [targetId: string, password: string | null];
}>();

// État local
const targetId = ref('');
const password = ref('');

// Méthodes
function handleSubmit() {
  if (!targetId.value.trim()) return;

  emit('connect', targetId.value.trim(), password.value.trim() || null);
}

function showHelp() {
  alert('Documentation complète disponible dans README.md');
}

function openSettings() {
  // TODO: Émettre un événement pour ouvrir les settings
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
