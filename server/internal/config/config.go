package config

import (
	"os"
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
	return &Config{
		Host:              getEnv("SERVER_HOST", ":8443"),
		CertFile:          getEnv("CERT_FILE", "certs/server.crt"),
		KeyFile:           getEnv("KEY_FILE", "certs/server.key"),
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
