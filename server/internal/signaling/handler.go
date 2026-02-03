package signaling

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/gorilla/websocket"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/models"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  4096,
	WriteBufferSize: 4096,
	CheckOrigin: func(r *http.Request) bool {
		// En production, vérifier l'origine de manière stricte
		// Pour le développement, accepter toutes les origines
		return true
	},
}

// HandleWebSocket gère les connexions WebSocket
func HandleWebSocket(hub *Hub, w http.ResponseWriter, r *http.Request) {
	// Mettre à niveau la connexion HTTP vers WebSocket
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("[HANDLER] Erreur d'upgrade WebSocket: %v", err)
		return
	}

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
	data, _ := json.Marshal(registerMsg.Data)
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
		ID:   deviceID,
		Conn: conn,
		Hub:  hub,
		Send: make(chan []byte, 256),
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
	confirmData, _ := json.Marshal(confirmMsg)
	client.Send <- confirmData

	// Démarrer les goroutines de lecture et d'écriture
	go client.WritePump()
	go client.ReadPump()
}
