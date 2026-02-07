package signaling

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/config"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/models"
)

// Constantes de configuration
const (
	MaxMessageSize  = 10 * 1024 * 1024 // 10 MB
	ReadBufferSize  = 4096
	WriteBufferSize = 4096
)

// newUpgrader crée un websocket.Upgrader avec les origines autorisées
func newUpgrader(allowedOrigins []string, disableOriginCheck bool) websocket.Upgrader {
	return websocket.Upgrader{
		ReadBufferSize:  ReadBufferSize,
		WriteBufferSize: WriteBufferSize,
		Error: func(w http.ResponseWriter, r *http.Request, status int, reason error) {
			log.Printf("[WS] Erreur WebSocket: %v", reason)
			http.Error(w, reason.Error(), status)
		},
		CheckOrigin: func(r *http.Request) bool {
			// Mode développement: accepter toutes les origines
			if disableOriginCheck {
				origin := r.Header.Get("Origin")
				log.Printf("[WS] ⚠️  Origine acceptée (vérification désactivée): %s", origin)
				return true
			}

			// Mode production: vérifier la whitelist
			origin := r.Header.Get("Origin")
			for _, allowed := range allowedOrigins {
				if origin == allowed {
					log.Printf("[WS] Origine autorisée: %s", origin)
					return true
				}
			}

			log.Printf("[WS] ❌ Origine refusée: %s (autorisées: %v)", origin, allowedOrigins)
			return false
		},
	}
}

// HandleWebSocket gère les connexions WebSocket
func HandleWebSocket(hub *Hub, cfg *config.Config, w http.ResponseWriter, r *http.Request) {
	upgrader := newUpgrader(cfg.AllowedOrigins, cfg.DisableOriginCheck)
	// Mettre à niveau la connexion HTTP vers WebSocket
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("[HANDLER] Erreur d'upgrade WebSocket: %v", err)
		return
	}

	// Définir la limite de taille de message
	// Cela empêche les attaques DoS via messages géants
	conn.SetReadLimit(MaxMessageSize)

	// Lire le message d'enregistrement
	var registerMsg models.Message
	if err := conn.ReadJSON(&registerMsg); err != nil {
		log.Printf("[HANDLER] Erreur de lecture du message d'enregistrement: %v", err)
		conn.Close()
		return
	}

	// Vérifier que c'est bien un message d'enregistrement
	if registerMsg.Type != models.TypeRegister {
		log.Printf("[HANDLER] Premier message n'est pas Register: %s", registerMsg.Type)
		conn.Close()
		return
	}

	// Extraire l'ID du dispositif
	data, err := json.Marshal(registerMsg.Data)
	if err != nil {
		log.Printf("[HANDLER] Erreur marshal RegisterMessage data: %v", err)
		conn.Close()
		return
	}
	var regData models.RegisterMessage
	if err := json.Unmarshal(data, &regData); err != nil {
		log.Printf("[HANDLER] Erreur parsing RegisterMessage: %v", err)
		conn.Close()
		return
	}

	deviceID := regData.DeviceID
	if deviceID == "" {
		log.Printf("[HANDLER] Device ID vide")
		conn.Close()
		return
	}

	log.Printf("[HANDLER] Nouveau client connecté: %s", deviceID)

	// Créer le client
	client := &Client{
		ID:               deviceID,
		Conn:             conn,
		Hub:              hub,
		Send:             make(chan []byte, 256),
		lastResetTime:    time.Now(),
		messageCount:     0,
		maxMessagesPerMin: 100, // Limite : 100 messages par minute
	}

	// Enregistrer le client auprès du hub
	hub.register <- client

	// Envoyer une confirmation d'enregistrement
	confirmMsg := models.Message{
		Type: models.TypeRegister,
		Data: map[string]interface{}{
			"success": true,
			"message": "Enregistrement réussi",
		},
	}
	confirmData, err := json.Marshal(confirmMsg)
	if err != nil {
		log.Printf("[HANDLER] Erreur marshal confirmation: %v", err)
		return
	}
	select {
	case client.Send <- confirmData:
	default:
		log.Printf("[HANDLER] Canal saturé pour %s, confirmation non envoyée", deviceID)
	}

	// Démarrer les goroutines de lecture et d'écriture
	go client.WritePump()
	go client.ReadPump()
}
