package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// Config contient toute la configuration du serveur
type Config struct {
	// Adresse d'écoute du serveur
	Host string

	// Chemins des certificats TLS
	CertFile string
	KeyFile  string

	// Forcer TLS en production (par défaut: true)
	RequireTLS bool

	// Auto-générer certificats auto-signés si absents (dev uniquement)
	AutoGenerateCerts bool

	// Niveau de logging (debug, info, warn, error)
	LogLevel string

	// Nombre maximum de clients simultanés
	MaxClients int

	// Timeout de connexion en secondes
	ConnectionTimeout int

	// Origines WebSocket autorisées (CheckOrigin)
	AllowedOrigins []string

	// Désactiver la vérification d'origine (DÉVELOPPEMENT UNIQUEMENT)
	DisableOriginCheck bool
}

// LoadFromEnv charge la configuration depuis les variables d'environnement
func LoadFromEnv() *Config {
	// Utiliser chemin absolu basé sur l'exécutable pour les certificats
	exePath, err := os.Executable()
	var exeDir string
	if err == nil {
		exeDir = filepath.Dir(exePath)
	} else {
		exeDir, _ = os.Getwd()
	}

	// Chemins optionnels pour HTTPS (relatifs au dossier de l'exécutable)
	defaultCertFile := filepath.Join(exeDir, "cert.pem")
	defaultKeyFile := filepath.Join(exeDir, "key.pem")
	// Ne pas utiliser les chemins par défaut si les fichiers n'existent pas
	if _, err := os.Stat(defaultCertFile); os.IsNotExist(err) {
		defaultCertFile = ""
	}
	if _, err := os.Stat(defaultKeyFile); os.IsNotExist(err) {
		defaultKeyFile = ""
	}

	// Origines autorisées par défaut
	defaultOrigins := []string{
		"http://localhost:9000",
		"http://127.0.0.1:9000",
		"http://localhost:1420",
		"http://127.0.0.1:1420",
		"tauri://localhost",
	}

	allowedOrigins := defaultOrigins
	if originsEnv := os.Getenv("ALLOWED_ORIGINS"); originsEnv != "" {
		allowedOrigins = []string{}
		for _, origin := range strings.Split(originsEnv, ",") {
			origin = strings.TrimSpace(origin)
			if origin != "" {
				allowedOrigins = append(allowedOrigins, origin)
			}
		}
	}

	return &Config{
		Host:               getEnv("SERVER_HOST", ":9000"),
		CertFile:           getEnv("CERT_FILE", defaultCertFile),
		KeyFile:            getEnv("KEY_FILE", defaultKeyFile),
		RequireTLS:         getEnvAsBool("REQUIRE_TLS", true),
		AutoGenerateCerts:  getEnvAsBool("AUTO_GENERATE_CERTS", false),
		LogLevel:           getEnv("LOG_LEVEL", "info"),
		MaxClients:         getEnvAsInt("MAX_CLIENTS", 1000),
		ConnectionTimeout:  getEnvAsInt("CONNECTION_TIMEOUT", 60),
		AllowedOrigins:     allowedOrigins,
		DisableOriginCheck: getEnvAsBool("DISABLE_ORIGIN_CHECK", false),
	}
}

// Helpers pour lire les variables d'environnement
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getEnvAsInt(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if intValue, err := strconv.Atoi(value); err == nil {
			return intValue
		}
	}
	return defaultValue
}

func getEnvAsBool(key string, defaultValue bool) bool {
	if value := os.Getenv(key); value != "" {
		switch strings.ToLower(value) {
		case "true", "1", "yes":
			return true
		case "false", "0", "no":
			return false
		}
	}
	return defaultValue
}

// Validate vérifie la validité de la configuration
func (c *Config) Validate() error {
	if c.Host == "" {
		return fmt.Errorf("HOST ne peut pas être vide")
	}
	if c.MaxClients <= 0 {
		return fmt.Errorf("MAX_CLIENTS doit être > 0, reçu: %d", c.MaxClients)
	}
	if c.ConnectionTimeout <= 0 {
		return fmt.Errorf("CONNECTION_TIMEOUT doit être > 0, reçu: %d", c.ConnectionTimeout)
	}

	// SÉCURITÉ CRITIQUE: Vérifier TLS en mode production
	if c.RequireTLS {
		if c.CertFile == "" || c.KeyFile == "" {
			if c.AutoGenerateCerts {
				// Auto-génération activée, les certificats seront créés au démarrage
				return nil
			}
			return fmt.Errorf("TLS OBLIGATOIRE: Certificats manquants. Définissez CERT_FILE/KEY_FILE ou activez AUTO_GENERATE_CERTS=true pour le dev")
		}
		// Vérifier que les fichiers existent
		if _, err := os.Stat(c.CertFile); os.IsNotExist(err) {
			return fmt.Errorf("fichier certificat introuvable: %s", c.CertFile)
		}
		if _, err := os.Stat(c.KeyFile); os.IsNotExist(err) {
			return fmt.Errorf("fichier clé privée introuvable: %s", c.KeyFile)
		}
	}

	return nil
}
