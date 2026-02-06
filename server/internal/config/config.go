package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
)

// Config contient toute la configuration du serveur
type Config struct {
	// Adresse d'écoute du serveur
	Host string

	// Chemins des certificats TLS
	CertFile string
	KeyFile  string

	// Niveau de logging (debug, info, warn, error)
	LogLevel string

	// Nombre maximum de clients simultanés
	MaxClients int

	// Timeout de connexion en secondes
	ConnectionTimeout int
}

// LoadFromEnv charge la configuration depuis les variables d'environnement
func LoadFromEnv() *Config {
	// Utiliser chemin absolu basé sur l'exécutable pour les certificats
	exePath, err := os.Executable()
	var exeDir string
	if err == nil {
		exeDir = filepath.Dir(exePath)
	} else {
		// Fallback sur le répertoire de travail actuel
		exeDir, _ = os.Getwd()
	}

	// Chemins optionnels pour HTTPS (non requis pour HTTP simple)
	// Si vous voulez activer HTTPS, définissez CERT_FILE et KEY_FILE dans l'environnement
	defaultCertFile := ""
	defaultKeyFile := ""

	return &Config{
		Host:              getEnv("SERVER_HOST", ":9000"),
		CertFile:          getEnv("CERT_FILE", defaultCertFile),
		KeyFile:           getEnv("KEY_FILE", defaultKeyFile),
		LogLevel:          getEnv("LOG_LEVEL", "info"),
		MaxClients:        getEnvAsInt("MAX_CLIENTS", 1000),
		ConnectionTimeout: getEnvAsInt("CONNECTION_TIMEOUT", 60),
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
	// Vérifier que les fichiers de certificats existent si fournis
	if c.CertFile != "" {
		if _, err := os.Stat(c.CertFile); os.IsNotExist(err) {
			return fmt.Errorf("fichier certificat introuvable: %s", c.CertFile)
		}
	}
	if c.KeyFile != "" {
		if _, err := os.Stat(c.KeyFile); os.IsNotExist(err) {
			return fmt.Errorf("fichier clé privée introuvable: %s", c.KeyFile)
		}
	}
	return nil
}
