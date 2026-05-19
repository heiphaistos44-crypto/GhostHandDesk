package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/config"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/signaling"
	"github.com/joho/godotenv"
)

func main() {
	// Charger les variables d'environnement depuis .env (optionnel)
	if err := godotenv.Load(); err != nil {
		log.Println("[MAIN] Aucun fichier .env trouvé, utilisation des valeurs par défaut")
	}

	// Charger la configuration
	cfg := config.LoadFromEnv()
	log.Printf("[MAIN] Configuration chargée: Host=%s, CertFile=%s, MaxClients=%d",
		cfg.Host, cfg.CertFile, cfg.MaxClients)

	// Valider la configuration
	if err := cfg.Validate(); err != nil {
		log.Fatalf("[MAIN] Configuration invalide: %v", err)
	}
	log.Println("[MAIN] Configuration validée avec succès")

	// Créer et démarrer le hub
	hub := signaling.NewHub()
	go hub.Run()
	log.Println("[MAIN] Hub de signalement démarré")

	// Stocker le temps de démarrage pour calculer l'uptime
	startTime := time.Now()

	// Configurer les routes HTTP
	mux := http.NewServeMux()

	// Route WebSocket pour la signalisation
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		signaling.HandleWebSocket(hub, w, r)
	})

	// Route de health check
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(map[string]any{
			"status":  "healthy",
			"clients": hub.GetClientCount(),
		})
	})

	// Route de statistiques
	mux.HandleFunc("/stats", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"total_clients": hub.GetClientCount(),
			"uptime":        time.Since(startTime).String(),
			"max_clients":   cfg.MaxClients,
		})
	})

	// Créer le serveur HTTPS
	server := &http.Server{
		Addr:         cfg.Host,
		Handler:      mux,
		ReadTimeout:  time.Duration(cfg.ConnectionTimeout) * time.Second,
		WriteTimeout: time.Duration(cfg.ConnectionTimeout) * time.Second,
	}

	// Canal pour gérer l'arrêt gracieux
	shutdown := make(chan os.Signal, 1)
	signal.Notify(shutdown, os.Interrupt, syscall.SIGTERM)

	// Determine TLS mode
	tlsEnabled := cfg.CertFile != "" && cfg.KeyFile != ""

	// Démarrer le serveur dans une goroutine
	go func() {
		log.Printf("[MAIN] Serveur de signalement démarré sur %s", cfg.Host)
		log.Println("[MAIN] Routes disponibles:")
		if tlsEnabled {
			log.Printf("  - wss://localhost%s/ws (WebSocket sécurisé TLS)", cfg.Host)
			log.Printf("  - https://localhost%s/health (Health check)", cfg.Host)
			log.Printf("  - https://localhost%s/stats (Statistiques)", cfg.Host)
			if err := server.ListenAndServeTLS(cfg.CertFile, cfg.KeyFile); err != nil && err != http.ErrServerClosed {
				log.Fatalf("[MAIN] Erreur démarrage TLS: %v", err)
			}
		} else {
			log.Printf("  - ws://localhost%s/ws (WebSocket — TLS désactivé, développement uniquement)", cfg.Host)
			log.Printf("  - http://localhost%s/health (Health check)", cfg.Host)
			log.Printf("  - http://localhost%s/stats (Statistiques)", cfg.Host)
			log.Println("[WARN] TLS désactivé. Définissez CERT_FILE et KEY_FILE pour activer HTTPS/WSS en production.")
			if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
				log.Fatalf("[MAIN] Erreur de démarrage du serveur: %v", err)
			}
		}
	}()

	// Attendre le signal d'arrêt
	<-shutdown
	log.Println("[MAIN] Signal d'arrêt reçu, fermeture gracieuse...")

	// Créer un contexte avec timeout pour l'arrêt
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Arrêter le serveur proprement
	if err := server.Shutdown(ctx); err != nil {
		log.Printf("[MAIN] Erreur lors de l'arrêt: %v", err)
	}

	log.Println("[MAIN] Serveur arrêté proprement")
}
