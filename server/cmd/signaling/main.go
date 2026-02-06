package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"sync"
	"syscall"
	"time"

	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/config"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/signaling"
	"github.com/joho/godotenv"
)

// simpleRateLimiter implémente un rate limiter basique par IP
type simpleRateLimiter struct {
	mu       sync.Mutex
	requests map[string][]time.Time
	limit    int
	window   time.Duration
}

func newSimpleRateLimiter(limit int, window time.Duration) *simpleRateLimiter {
	return &simpleRateLimiter{
		requests: make(map[string][]time.Time),
		limit:    limit,
		window:   window,
	}
}

func (rl *simpleRateLimiter) allow(ip string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	cutoff := now.Add(-rl.window)

	// Nettoyer les anciennes requêtes
	reqs := rl.requests[ip]
	valid := reqs[:0]
	for _, t := range reqs {
		if t.After(cutoff) {
			valid = append(valid, t)
		}
	}

	if len(valid) >= rl.limit {
		rl.requests[ip] = valid
		return false
	}

	rl.requests[ip] = append(valid, now)
	return true
}

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

	// Rate limiter pour les endpoints HTTP (30 req/min par IP)
	httpLimiter := newSimpleRateLimiter(30, time.Minute)

	// Configurer les routes HTTP
	mux := http.NewServeMux()

	// Route WebSocket pour la signalisation
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		signaling.HandleWebSocket(hub, cfg, w, r)
	})

	// Route de health check
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		if !httpLimiter.allow(r.RemoteAddr) {
			http.Error(w, "Too Many Requests", http.StatusTooManyRequests)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		if err := json.NewEncoder(w).Encode(map[string]interface{}{
			"status":  "healthy",
			"clients": hub.GetClientCount(),
		}); err != nil {
			log.Printf("[MAIN] Erreur encodage health: %v", err)
		}
	})

	// Route de statistiques avec pagination des clients
	mux.HandleFunc("/stats", func(w http.ResponseWriter, r *http.Request) {
		if !httpLimiter.allow(r.RemoteAddr) {
			http.Error(w, "Too Many Requests", http.StatusTooManyRequests)
			return
		}

		// Parser les paramètres de pagination
		page := 1
		perPage := 50
		if p := r.URL.Query().Get("page"); p != "" {
			if v, err := strconv.Atoi(p); err == nil && v >= 1 {
				page = v
			}
		}
		if pp := r.URL.Query().Get("per_page"); pp != "" {
			if v, err := strconv.Atoi(pp); err == nil && v >= 1 && v <= 100 {
				perPage = v
			}
		}

		// Récupérer tous les IDs et paginer
		allIDs := hub.GetClientIDs()
		totalClients := len(allIDs)
		totalPages := (totalClients + perPage - 1) / perPage
		if totalPages == 0 {
			totalPages = 1
		}

		// Calculer les indices de pagination
		start := (page - 1) * perPage
		end := start + perPage
		if start > totalClients {
			start = totalClients
		}
		if end > totalClients {
			end = totalClients
		}

		pagedIDs := allIDs[start:end]

		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(map[string]interface{}{
			"total_clients":     totalClients,
			"connected_clients": pagedIDs,
			"page":              page,
			"per_page":          perPage,
			"total_pages":       totalPages,
			"uptime":            time.Since(startTime).String(),
			"max_clients":       cfg.MaxClients,
		}); err != nil {
			log.Printf("[MAIN] Erreur encodage stats: %v", err)
		}
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

	// Démarrer le serveur dans une goroutine
	go func() {
		log.Printf("[MAIN] Serveur de signalement démarré sur %s", cfg.Host)
		log.Println("[MAIN] Routes disponibles:")
		log.Printf("  - ws://localhost%s/ws (WebSocket)", cfg.Host)
		log.Printf("  - http://localhost%s/health (Health check)", cfg.Host)
		log.Printf("  - http://localhost%s/stats (Statistiques)", cfg.Host)

		// Démarrer en HTTPS si les certificats sont fournis, sinon HTTP
		var err error
		if cfg.CertFile != "" && cfg.KeyFile != "" {
			log.Printf("[MAIN] Démarrage en mode HTTPS (cert: %s)", cfg.CertFile)
			err = server.ListenAndServeTLS(cfg.CertFile, cfg.KeyFile)
		} else {
			log.Println("[MAIN] Démarrage en mode HTTP (pas de certificats TLS)")
			err = server.ListenAndServe()
		}
		if err != nil && err != http.ErrServerClosed {
			log.Fatalf("[MAIN] Erreur de démarrage du serveur: %v", err)
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
