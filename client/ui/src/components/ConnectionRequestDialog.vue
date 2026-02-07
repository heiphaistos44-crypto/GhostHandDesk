<template>
  <div v-if="visible" class="dialog-overlay" @click="reject">
    <div class="dialog-container" @click.stop>
      <div class="dialog-header">
        <h2>🔔 Demande de Connexion</h2>
      </div>

      <div class="dialog-content">
        <div class="request-icon">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <circle cx="12" cy="12" r="10" stroke-width="2"/>
            <path d="M12 16v-4M12 8h.01" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>

        <p class="request-message">
          <strong>{{ requestFrom }}</strong> demande à se connecter à votre ordinateur.
        </p>

        <p class="request-warning">
          ⚠️ Cette personne pourra voir votre écran et contrôler votre souris et clavier.
        </p>

        <div class="request-info">
          <div class="info-item">
            <span class="info-label">ID de l'appareil :</span>
            <code class="info-value">{{ requestFrom }}</code>
          </div>
          <div class="info-item">
            <span class="info-label">Heure :</span>
            <span class="info-value">{{ formattedTime }}</span>
          </div>
        </div>
      </div>

      <div class="dialog-actions">
        <button class="btn btn-reject" @click="reject">
          ❌ Refuser
        </button>
        <button class="btn btn-accept" @click="accept">
          ✅ Accepter
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  visible: boolean;
  requestFrom: string;
  timestamp: number;
}>();

const emit = defineEmits<{
  accept: [];
  reject: [];
}>();

const formattedTime = computed(() => {
  const date = new Date(props.timestamp * 1000);
  return date.toLocaleTimeString('fr-FR');
});

const accept = () => {
  emit('accept');
};

const reject = () => {
  emit('reject');
};
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.dialog-container {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 16px;
  padding: 0;
  max-width: 500px;
  width: 90%;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  animation: slideIn 0.3s ease-out;
  overflow: hidden;
}

@keyframes slideIn {
  from {
    transform: translateY(-50px) scale(0.9);
    opacity: 0;
  }
  to {
    transform: translateY(0) scale(1);
    opacity: 1;
  }
}

.dialog-header {
  background: rgba(255, 255, 255, 0.1);
  padding: 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

.dialog-header h2 {
  margin: 0;
  color: white;
  font-size: 24px;
  font-weight: 600;
}

.dialog-content {
  padding: 30px;
  background: white;
}

.request-icon {
  text-align: center;
  color: #667eea;
  margin-bottom: 20px;
}

.request-message {
  font-size: 18px;
  color: #333;
  text-align: center;
  margin-bottom: 15px;
}

.request-message strong {
  color: #667eea;
  font-weight: 600;
}

.request-warning {
  background: #fff3cd;
  border: 1px solid #ffc107;
  border-radius: 8px;
  padding: 12px;
  color: #856404;
  font-size: 14px;
  text-align: center;
  margin-bottom: 20px;
}

.request-info {
  background: #f8f9fa;
  border-radius: 8px;
  padding: 15px;
  margin-bottom: 10px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.info-item:last-child {
  margin-bottom: 0;
}

.info-label {
  font-weight: 500;
  color: #666;
  font-size: 14px;
}

.info-value {
  color: #333;
  font-size: 14px;
}

.info-value code {
  background: #e9ecef;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: 'Consolas', monospace;
  font-size: 12px;
}

.dialog-actions {
  display: flex;
  gap: 12px;
  padding: 20px;
  background: white;
}

.btn {
  flex: 1;
  padding: 14px 24px;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.btn:active {
  transform: translateY(0);
}

.btn-reject {
  background: #dc3545;
  color: white;
}

.btn-reject:hover {
  background: #c82333;
}

.btn-accept {
  background: #28a745;
  color: white;
}

.btn-accept:hover {
  background: #218838;
}
</style>
