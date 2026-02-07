package main

import (
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/json"
	"encoding/pem"
	"log"
	"math/big"
	"net"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
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

// generateSelfSignedCert génère des certificats auto-signés pour le développement
func generateSelfSignedCert(certPath, keyPath string) error {
	log.Println("[CERT] Génération de certificats auto-signés pour le développement...")

	// Générer une clé privée ECDSA
	priv, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		return err
	}

	// Créer le template de certificat
	serialNumber, err := rand.Int(rand.Reader, new(big.Int).Lsh(big.NewInt(1), 128))
	if err != nil {
		return err
	}

	template := x509.Certificate{
		SerialNumber: serialNumber,
		Subject: pkix.Name{
			Organization: []string{"GhostHandDesk Dev"},
			CommonName:   "localhost",
		},
		NotBefore:             time.Now(),
		NotAfter:              time.Now().Add(365 * 24 * time.Hour), // 1 an
		KeyUsage:              x509.KeyUsageKeyEncipherment | x509.KeyUsageDigitalSignature,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		BasicConstraintsValid: true,
		DNSNames:              []string{"localhost"},
		IPAddresses:           []net.IP{net.ParseIP("127.0.0.1")},
	}

	// Créer le certificat
	derBytes, err := x509.CreateCertificate(rand.Reader, &template, &template, &priv.PublicKey, priv)
	if err != nil {
		return err
	}

	// Créer le dossier certs s'il n'existe pas
	certsDir := filepath.Dir(certPath)
	if err := os.MkdirAll(certsDir, 0755); err != nil {
		return err
	}

	// Écrire le certificat
	certOut, err := os.Create(certPath)
	if err != nil {
		return err
	}
	defer certOut.Close()
	if err := pem.Encode(certOut, &pem.Block{Type: "CERTIFICATE", Bytes: derBytes}); err != nil {
		return err
	}

	// Écrire la clé privée
	keyOut, err := os.Create(keyPath)
	if err != nil {
		return err
	}
	defer keyOut.Close()
	privBytes, err := x509.MarshalECPrivateKey(priv)
	if err != nil {
		return err
	}
	if err := pem.Encode(keyOut, &pem.Block{Type: "EC PRIVATE KEY", Bytes: privBytes}); err != nil {
		return err
	}

	log.Printf("[CERT] ✅ Certificats générés: %s, %s", certPath, keyPath)
	log.Println("[CERT] ⚠️  AVERTISSEMENT: Certificats auto-signés - À UTILISER EN DÉVELOPPEMENT UNIQUEMENT")

	return nil
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

	// Auto-génération de certificats si nécessaire
	if cfg.RequireTLS && cfg.AutoGenerateCerts {
		if cfg.CertFile == "" || cfg.KeyFile == "" {
			// Chemins par défaut pour les certificats auto-générés
			exePath, _ := os.Executable()
			exeDir := filepath.Dir(exePath)
			certsDir := filepath.Join(exeDir, "certs")
			cfg.CertFile = filepath.Join(certsDir, "cert.pem")
			cfg.KeyFile = filepath.Join(certsDir, "key.pem")
		}

		// Vérifier si les certificats existent déjà
		_, certErr := os.Stat(cfg.CertFile)
		_, keyErr := os.Stat(cfg.KeyFile)
		if os.IsNotExist(certErr) || os.IsNotExist(keyErr) {
			if err := generateSelfSignedCert(cfg.CertFile, cfg.KeyFile); err != nil {
				log.Fatalf("[MAIN] Erreur génération certificats: %v", err)
			}
		} else {
			log.Printf("[CERT] Certificats existants trouvés: %s", cfg.CertFile)
		}
	}

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

		// Démarrer en HTTPS (obligatoire en production)
		var err error
		if cfg.RequireTLS {
			if cfg.CertFile == "" || cfg.KeyFile == "" {
				log.Fatal("[MAIN] ❌ ERREUR CRITIQUE: TLS obligatoire mais certificats manquants")
			}
			log.Println("[MAIN] 🔒 Mode HTTPS activé (TLS obligatoire)")
			log.Println("[MAIN] Routes disponibles:")
			log.Printf("  - wss://localhost%s/ws (WebSocket sécurisé)", cfg.Host)
			log.Printf("  - https://localhost%s/health (Health check)", cfg.Host)
			log.Printf("  - https://localhost%s/stats (Statistiques)", cfg.Host)
			log.Printf("[MAIN] 📜 Certificat: %s", cfg.CertFile)

			err = server.ListenAndServeTLS(cfg.CertFile, cfg.KeyFile)
		} else {
			log.Println("[MAIN] ⚠️  Mode HTTP activé (TLS désactivé - DÉVELOPPEMENT UNIQUEMENT)")
			log.Println("[MAIN] Routes disponibles:")
			log.Printf("  - ws://localhost%s/ws (WebSocket)", cfg.Host)
			log.Printf("  - http://localhost%s/health (Health check)", cfg.Host)
			log.Printf("  - http://localhost%s/stats (Statistiques)", cfg.Host)

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
