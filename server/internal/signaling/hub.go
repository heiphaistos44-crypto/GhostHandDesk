package signaling

import (
	"encoding/json"
	"log"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/heiphaistos44-crypto/GhostHandDesk/server/internal/models"
)

// Hub gère tous les clients connectés et route les messages
type Hub struct {
	// Clients enregistrés
	clients map[string]*Client

	// Canal pour enregistrer un nouveau client
	register chan *Client

	// Canal pour désenregistrer un client
	unregister chan *Client

	// Canal pour diffuser des messages
	broadcast chan *BroadcastMessage

	// Mutex pour protéger l'accès concurrent
	mu sync.RWMutex
}

// Client représente un client WebSocket connecté
type Client struct {
	ID   string
	Conn *websocket.Conn
	Hub  *Hub
	Send chan []byte
}

// BroadcastMessage représente un message à diffuser
type BroadcastMessage struct {
	To      string
	Message []byte
}

// NewHub crée un nouveau hub
func NewHub() *Hub {
	return &Hub{
		clients:    make(map[string]*Client),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		broadcast:  make(chan *BroadcastMessage),
	}
}

// Run démarre la boucle principale du hub
func (h *Hub) Run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client.ID] = client
			h.mu.Unlock()
			log.Printf("[HUB] Client enregistré: %s (Total: %d)", client.ID, len(h.clients))

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client.ID]; ok {
				delete(h.clients, client.ID)
				close(client.Send)
				log.Printf("[HUB] Client désenregistré: %s (Total: %d)", client.ID, len(h.clients))
			}
			h.mu.Unlock()

		case msg := <-h.broadcast:
			h.mu.RLock()
			client, ok := h.clients[msg.To]
			h.mu.RUnlock()

			if ok {
				select {
				case client.Send <- msg.Message:
					// Message envoyé avec succès
				default:
					// Le canal est plein, déconnecter le client
					h.mu.Lock()
					close(client.Send)
					delete(h.clients, client.ID)
					h.mu.Unlock()
					log.Printf("[HUB] Client %s déconnecté (canal saturé)", client.ID)
				}
			} else {
				log.Printf("[HUB] Client destinataire %s non trouvé", msg.To)
			}
		}
	}
}

// SendToClient envoie un message à un client spécifique
func (h *Hub) SendToClient(to string, message interface{}) error {
	data, err := json.Marshal(message)
	if err != nil {
		return err
	}

	h.broadcast <- &BroadcastMessage{
		To:      to,
		Message: data,
	}

	return nil
}

// GetClientCount retourne le nombre de clients connectés
func (h *Hub) GetClientCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.clients)
}

// ReadPump pompe les messages du client vers le hub
func (c *Client) ReadPump() {
	defer func() {
		c.Hub.unregister <- c
		c.Conn.Close()
	}()

	c.Conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.Conn.SetPongHandler(func(string) error {
		c.Conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, message, err := c.Conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("[CLIENT] Erreur de lecture: %v", err)
			}
			break
		}

		// Parser le message
		var msg models.Message
		if err := json.Unmarshal(message, &msg); err != nil {
			log.Printf("[CLIENT] Erreur de parsing: %v", err)
			continue
		}

		// Traiter le message
		c.handleMessage(&msg)
	}
}

// WritePump pompe les messages du hub vers le client
func (c *Client) WritePump() {
	ticker := time.NewTicker(54 * time.Second)
	defer func() {
		ticker.Stop()
		c.Conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.Send:
			c.Conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if !ok {
				// Le hub a fermé le canal
				c.Conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := c.Conn.WriteMessage(websocket.TextMessage, message); err != nil {
				log.Printf("[CLIENT] Erreur d'écriture: %v", err)
				return
			}

		case <-ticker.C:
			c.Conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if err := c.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

// handleMessage traite un message reçu
func (c *Client) handleMessage(msg *models.Message) {
	log.Printf("[CLIENT %s] Message reçu: %s", c.ID, msg.Type)

	switch msg.Type {
	case models.TypeOffer:
		c.handleOffer(msg)
	case models.TypeAnswer:
		c.handleAnswer(msg)
	case models.TypeIceCandidate:
		c.handleIceCandidate(msg)
	case models.TypeConnectRequest:
		c.handleConnectRequest(msg)
	case models.TypePing:
		c.handlePing()
	default:
		log.Printf("[CLIENT] Type de message inconnu: %s", msg.Type)
	}
}

// handleOffer traite un message d'offre WebRTC
func (c *Client) handleOffer(msg *models.Message) {
	data, _ := json.Marshal(msg.Data)
	var offer models.OfferMessage
	if err := json.Unmarshal(data, &offer); err != nil {
		log.Printf("[CLIENT] Erreur parsing offer: %v", err)
		return
	}

	// Transférer l'offre au destinataire
	c.Hub.SendToClient(offer.To, msg)
}

// handleAnswer traite un message de réponse WebRTC
func (c *Client) handleAnswer(msg *models.Message) {
	data, _ := json.Marshal(msg.Data)
	var answer models.AnswerMessage
	if err := json.Unmarshal(data, &answer); err != nil {
		log.Printf("[CLIENT] Erreur parsing answer: %v", err)
		return
	}

	// Transférer la réponse au destinataire
	c.Hub.SendToClient(answer.To, msg)
}

// handleIceCandidate traite un candidat ICE
func (c *Client) handleIceCandidate(msg *models.Message) {
	data, _ := json.Marshal(msg.Data)
	var ice models.IceCandidateMessage
	if err := json.Unmarshal(data, &ice); err != nil {
		log.Printf("[CLIENT] Erreur parsing ICE: %v", err)
		return
	}

	// Transférer le candidat au destinataire
	c.Hub.SendToClient(ice.To, msg)
}

// handleConnectRequest traite une demande de connexion
func (c *Client) handleConnectRequest(msg *models.Message) {
	data, _ := json.Marshal(msg.Data)
	var req models.ConnectRequestMessage
	if err := json.Unmarshal(data, &req); err != nil {
		log.Printf("[CLIENT] Erreur parsing connect request: %v", err)
		return
	}

	log.Printf("[CLIENT %s] Demande de connexion vers %s", c.ID, req.TargetID)

	// Vérifier si le client cible existe
	c.Hub.mu.RLock()
	_, exists := c.Hub.clients[req.TargetID]
	c.Hub.mu.RUnlock()

	if !exists {
		// Envoyer un message de rejet
		response := models.Message{
			Type: models.TypeError,
			Data: models.ErrorMessage{
				Code:    404,
				Message: "Client cible non trouvé",
			},
		}
		data, _ := json.Marshal(response)
		c.Send <- data
		return
	}

	// Transférer la demande au client cible
	// Dans une vraie implémentation, on attendrait l'acceptation du client cible
	c.Hub.SendToClient(req.TargetID, msg)
}

// handlePing traite un message de ping
func (c *Client) handlePing() {
	response := models.Message{
		Type: models.TypePong,
	}
	data, _ := json.Marshal(response)
	c.Send <- data
}
