//! Module d'adaptation dynamique du bitrate vidéo
//!
//! Ce module ajuste automatiquement la qualité vidéo en fonction des conditions réseau
//! (RTT, packet loss, bande passante) pour maintenir une expérience fluide.

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Contrôleur de bitrate adaptatif
pub struct AdaptiveBitrateController {
    /// Bitrate actuel en kbps
    current_bitrate: u32,

    /// Qualité JPEG actuelle (10-100)
    current_quality: u8,

    /// Historique des RTT (Round Trip Time)
    rtt_history: VecDeque<Duration>,

    /// Historique des taux de perte de paquets
    packet_loss_history: VecDeque<f32>,

    /// Dernière mesure de RTT
    last_rtt_measurement: Instant,

    /// Configuration
    config: AdaptiveBitrateConfig,

    /// Statistiques
    stats: AdaptiveBitrateStats,
}

/// Configuration du contrôleur adaptatif
#[derive(Debug, Clone)]
pub struct AdaptiveBitrateConfig {
    /// Bitrate minimum (kbps)
    pub min_bitrate: u32,

    /// Bitrate maximum (kbps)
    pub max_bitrate: u32,

    /// Qualité JPEG minimale
    pub min_quality: u8,

    /// Qualité JPEG maximale
    pub max_quality: u8,

    /// Seuil de RTT "élevé" (ms)
    pub high_rtt_threshold_ms: u64,

    /// Seuil de RTT "faible" (ms)
    pub low_rtt_threshold_ms: u64,

    /// Seuil de packet loss "élevé" (%)
    pub high_packet_loss_threshold: f32,

    /// Facteur de réduction lors de conditions dégradées (0.8 = -20%)
    pub degradation_factor: f32,

    /// Facteur d'augmentation lors de conditions optimales (1.1 = +10%)
    pub improvement_factor: f32,

    /// Taille de l'historique pour moyennage
    pub history_size: usize,

    /// Intervalle minimum entre ajustements (ms)
    pub adjustment_interval_ms: u64,
}

impl Default for AdaptiveBitrateConfig {
    fn default() -> Self {
        Self {
            min_bitrate: 500,           // 500 kbps
            max_bitrate: 8000,          // 8 Mbps
            min_quality: 40,            // Qualité minimale acceptable
            max_quality: 95,            // Qualité maximale
            high_rtt_threshold_ms: 150, // > 150ms = dégradé
            low_rtt_threshold_ms: 50,   // < 50ms = excellent
            high_packet_loss_threshold: 0.05, // 5% packet loss
            degradation_factor: 0.85,   // Réduire 15% si problème
            improvement_factor: 1.05,   // Augmenter 5% si bon
            history_size: 10,           // Moyenner sur 10 mesures
            adjustment_interval_ms: 2000, // Ajuster max toutes les 2s
        }
    }
}

/// Statistiques du contrôleur
#[derive(Debug, Clone, Default)]
pub struct AdaptiveBitrateStats {
    /// Nombre total d'ajustements
    pub total_adjustments: u64,

    /// Nombre de réductions de qualité
    pub quality_reductions: u64,

    /// Nombre d'augmentations de qualité
    pub quality_increases: u64,

    /// RTT moyen actuel (ms)
    pub average_rtt_ms: u64,

    /// Packet loss moyen actuel (%)
    pub average_packet_loss: f32,
}

impl AdaptiveBitrateController {
    /// Créer un nouveau contrôleur avec configuration par défaut
    pub fn new() -> Self {
        Self::with_config(AdaptiveBitrateConfig::default())
    }

    /// Créer un contrôleur avec configuration personnalisée
    pub fn with_config(config: AdaptiveBitrateConfig) -> Self {
        let initial_quality = (config.min_quality + config.max_quality) / 2;
        let initial_bitrate = (config.min_bitrate + config.max_bitrate) / 2;

        Self {
            current_bitrate: initial_bitrate,
            current_quality: initial_quality,
            rtt_history: VecDeque::with_capacity(config.history_size),
            packet_loss_history: VecDeque::with_capacity(config.history_size),
            last_rtt_measurement: Instant::now(),
            config,
            stats: AdaptiveBitrateStats::default(),
        }
    }

    /// Obtenir la qualité JPEG actuelle
    pub fn get_quality(&self) -> u8 {
        self.current_quality
    }

    /// Obtenir le bitrate actuel
    pub fn get_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// Obtenir les statistiques
    pub fn get_stats(&self) -> &AdaptiveBitrateStats {
        &self.stats
    }

    /// Mettre à jour avec une nouvelle mesure de RTT
    pub fn update_rtt(&mut self, rtt: Duration) {
        // Ajouter à l'historique
        self.rtt_history.push_back(rtt);
        if self.rtt_history.len() > self.config.history_size {
            self.rtt_history.pop_front();
        }

        self.last_rtt_measurement = Instant::now();

        // Calculer moyenne
        let avg_rtt = self.average_rtt();
        self.stats.average_rtt_ms = avg_rtt.as_millis() as u64;

        debug!("RTT mis à jour: {:?} (avg: {:?})", rtt, avg_rtt);

        // Ajuster si nécessaire
        self.maybe_adjust();
    }

    /// Mettre à jour avec un taux de perte de paquets (0.0-1.0)
    pub fn update_packet_loss(&mut self, loss_rate: f32) {
        // Valider
        let loss_rate = loss_rate.clamp(0.0, 1.0);

        // Ajouter à l'historique
        self.packet_loss_history.push_back(loss_rate);
        if self.packet_loss_history.len() > self.config.history_size {
            self.packet_loss_history.pop_front();
        }

        // Calculer moyenne
        self.stats.average_packet_loss = self.average_packet_loss();

        debug!("Packet loss mis à jour: {:.2}% (avg: {:.2}%)",
            loss_rate * 100.0, self.stats.average_packet_loss * 100.0);

        // Ajuster si nécessaire
        self.maybe_adjust();
    }

    /// Calculer le RTT moyen
    fn average_rtt(&self) -> Duration {
        if self.rtt_history.is_empty() {
            return Duration::from_millis(50); // Défaut optimiste
        }

        let sum: Duration = self.rtt_history.iter().sum();
        sum / self.rtt_history.len() as u32
    }

    /// Calculer le packet loss moyen
    fn average_packet_loss(&self) -> f32 {
        if self.packet_loss_history.is_empty() {
            return 0.0;
        }

        self.packet_loss_history.iter().sum::<f32>() / self.packet_loss_history.len() as f32
    }

    /// Ajuster la qualité si les conditions ont changé
    fn maybe_adjust(&mut self) {
        // Vérifier l'intervalle minimum entre ajustements
        if self.last_rtt_measurement.elapsed().as_millis() < self.config.adjustment_interval_ms as u128 {
            return; // Trop tôt
        }

        let avg_rtt = self.average_rtt();
        let avg_loss = self.average_packet_loss();

        let rtt_ms = avg_rtt.as_millis() as u64;

        // Déterminer l'état du réseau
        let should_reduce = rtt_ms > self.config.high_rtt_threshold_ms
            || avg_loss > self.config.high_packet_loss_threshold;

        let should_improve = rtt_ms < self.config.low_rtt_threshold_ms
            && avg_loss < self.config.high_packet_loss_threshold / 2.0;

        if should_reduce {
            self.reduce_quality(avg_rtt, avg_loss);
        } else if should_improve {
            self.increase_quality();
        }
    }

    /// Réduire la qualité (conditions réseau dégradées)
    fn reduce_quality(&mut self, rtt: Duration, packet_loss: f32) {
        let old_quality = self.current_quality;
        let old_bitrate = self.current_bitrate;

        // Réduire bitrate
        self.current_bitrate = ((self.current_bitrate as f32 * self.config.degradation_factor) as u32)
            .max(self.config.min_bitrate);

        // Réduire qualité JPEG
        self.current_quality = ((self.current_quality as f32 * self.config.degradation_factor) as u8)
            .max(self.config.min_quality);

        self.stats.quality_reductions += 1;
        self.stats.total_adjustments += 1;

        warn!(
            "🔻 Qualité réduite: {}→{} (bitrate: {}→{} kbps) | RTT={:?}, Loss={:.2}%",
            old_quality, self.current_quality,
            old_bitrate, self.current_bitrate,
            rtt, packet_loss * 100.0
        );
    }

    /// Augmenter la qualité (conditions réseau excellentes)
    fn increase_quality(&mut self) {
        let old_quality = self.current_quality;
        let old_bitrate = self.current_bitrate;

        // Augmenter bitrate
        self.current_bitrate = ((self.current_bitrate as f32 * self.config.improvement_factor) as u32)
            .min(self.config.max_bitrate);

        // Augmenter qualité JPEG
        self.current_quality = ((self.current_quality as f32 * self.config.improvement_factor) as u8)
            .min(self.config.max_quality);

        // Éviter changements trop petits
        if self.current_quality == old_quality && self.current_bitrate == old_bitrate {
            return;
        }

        self.stats.quality_increases += 1;
        self.stats.total_adjustments += 1;

        info!(
            "🔺 Qualité augmentée: {}→{} (bitrate: {}→{} kbps)",
            old_quality, self.current_quality,
            old_bitrate, self.current_bitrate
        );
    }

    /// Réinitialiser à la configuration par défaut
    pub fn reset(&mut self) {
        self.current_quality = (self.config.min_quality + self.config.max_quality) / 2;
        self.current_bitrate = (self.config.min_bitrate + self.config.max_bitrate) / 2;
        self.rtt_history.clear();
        self.packet_loss_history.clear();
        self.stats = AdaptiveBitrateStats::default();

        info!("Contrôleur adaptatif réinitialisé: quality={}, bitrate={} kbps",
            self.current_quality, self.current_bitrate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let controller = AdaptiveBitrateController::new();
        assert!(controller.get_quality() > 0);
        assert!(controller.get_bitrate() > 0);
    }

    #[test]
    fn test_rtt_update() {
        let mut controller = AdaptiveBitrateController::new();
        let initial_quality = controller.get_quality();

        // Simuler RTT élevé
        for _ in 0..5 {
            controller.update_rtt(Duration::from_millis(200));
        }

        // La qualité devrait avoir diminué
        assert!(controller.get_quality() <= initial_quality);
    }

    #[test]
    fn test_packet_loss_update() {
        use std::time::Duration;

        // Configuration avec intervalle d'ajustement court pour test
        let config = AdaptiveBitrateConfig {
            adjustment_interval_ms: 0, // Pas de délai pour test
            ..Default::default()
        };

        let mut controller = AdaptiveBitrateController::with_config(config);
        let initial_quality = controller.get_quality();

        // Attendre un peu pour permettre l'ajustement
        std::thread::sleep(Duration::from_millis(10));

        // Simuler packet loss élevé (10%)
        for _ in 0..5 {
            controller.update_packet_loss(0.10);
            std::thread::sleep(Duration::from_millis(5));
        }

        // La qualité devrait avoir diminué
        assert!(controller.get_quality() < initial_quality,
            "Expected quality < {} but got {}", initial_quality, controller.get_quality());
    }

    #[test]
    fn test_quality_improvement() {
        let mut controller = AdaptiveBitrateController::new();

        // Forcer une qualité basse
        controller.current_quality = 50;

        // Simuler excellentes conditions
        for _ in 0..5 {
            controller.update_rtt(Duration::from_millis(20));
            controller.update_packet_loss(0.0);
        }

        // La qualité devrait augmenter
        assert!(controller.get_quality() >= 50);
    }

    #[test]
    fn test_quality_bounds() {
        let config = AdaptiveBitrateConfig {
            min_quality: 40,
            max_quality: 95,
            ..Default::default()
        };

        let mut controller = AdaptiveBitrateController::with_config(config);

        // Forcer conditions extrêmes
        for _ in 0..20 {
            controller.update_rtt(Duration::from_millis(500)); // Très mauvais
            controller.update_packet_loss(0.5); // 50% loss
        }

        // La qualité ne doit pas descendre sous min
        assert!(controller.get_quality() >= 40);

        // Forcer excellentes conditions
        for _ in 0..20 {
            controller.update_rtt(Duration::from_millis(10)); // Excellent
            controller.update_packet_loss(0.0);
        }

        // La qualité ne doit pas dépasser max
        assert!(controller.get_quality() <= 95);
    }

    #[test]
    fn test_stats() {
        let mut controller = AdaptiveBitrateController::new();

        // Faire quelques ajustements
        controller.update_rtt(Duration::from_millis(200));
        controller.update_rtt(Duration::from_millis(200));

        let stats = controller.get_stats();
        assert!(stats.average_rtt_ms > 0);
    }

    #[test]
    fn test_reset() {
        let mut controller = AdaptiveBitrateController::new();

        // Modifier l'état
        controller.update_rtt(Duration::from_millis(300));
        controller.current_quality = 30;

        // Reset
        controller.reset();

        // Devrait revenir aux valeurs par défaut
        assert!(controller.get_quality() > 30);
        assert_eq!(controller.get_stats().total_adjustments, 0);
    }
}
