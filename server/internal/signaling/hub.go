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

	// pendingConnections tracks active password handshake pairs: targetID -> fromID
	pendingConnections map[string]string
	pcMu               sync.Mutex

	// relayPairs tracks bidirectional relay sessions: clientA → clientB et clientB → clientA
	relayPairs map[string]string
	relayMu    sync.Mutex
}

// Client représente un client WebSocket connecté
type Client struct {
	ID   string
	Conn *websocket.Conn
	Hub  *Hub
	Send chan []byte

	// Protection contre la double fermeture du canal Send
	closed bool

	// Rate limiting
	messageCount     int
	lastResetTime    time.Time
	rateLimitMu      sync.Mutex
	maxMessagesPerMin int
}

// BroadcastMessage représente un message à diffuser
type BroadcastMessage struct {
	To      string
	Message []byte
}

// NewHub crée un nouveau hub
func NewHub() *Hub {
	return &Hub{
		clients:            make(map[string]*Client),
		register:           make(chan *Client),
		unregister:         make(chan *Client),
		broadcast:          make(chan *BroadcastMessage),
		pendingConnections: make(map[string]string),
		relayPairs:         make(map[string]string),
	}
}

// RegisterRelayPair enregistre une session relay bidirectionnelle entre deux clients
func (h *Hub) RegisterRelayPair(id1, id2 string) {
	h.relayMu.Lock()
	h.relayPairs[id1] = id2
	h.relayPairs[id2] = id1
	h.relayMu.Unlock()
	log.Printf("[HUB] Relay pair enregistré: %s ↔ %s", id1, id2)
}

// GetRelayPartner retourne l'ID du partenaire relay d'un client, ou ("", false) si non trouvé
func (h *Hub) GetRelayPartner(clientID string) (string, bool) {
	h.relayMu.Lock()
	defer h.relayMu.Unlock()
	partner, ok := h.relayPairs[clientID]
	return partner, ok
}

// ClearRelayPair supprime la paire relay associée à un client (à la déconnexion)
func (h *Hub) ClearRelayPair(clientID string) {
	h.relayMu.Lock()
	if partner, ok := h.relayPairs[clientID]; ok {
		delete(h.relayPairs, clientID)
		delete(h.relayPairs, partner)
		log.Printf("[HUB] Relay pair supprimé: %s ↔ %s", clientID, partner)
	}
	h.relayMu.Unlock()
}

// RegisterPendingConnection enregistre une paire fromID→targetID en attente de handshake
func (h *Hub) RegisterPendingConnection(targetID, fromID string) {
	h.pcMu.Lock()
	h.pendingConnections[targetID] = fromID
	h.pcMu.Unlock()
}

// IsPeerAuthorized vérifie que senderID est bien partie de la paire en cours avec peerID
func (h *Hub) IsPeerAuthorized(senderID, peerID string) bool {
	h.pcMu.Lock()
	defer h.pcMu.Unlock()
	// Sens 1: sender est le "from", peer est le "target"
	if from, ok := h.pendingConnections[peerID]; ok && from == senderID {
		return true
	}
	// Sens 2: sender est le "target", peer est le "from"
	if from, ok := h.pendingConnections[senderID]; ok && from == peerID {
		return true
	}
	return false
}

// ClearPendingConnection supprime la paire après établissement ou timeout
func (h *Hub) ClearPendingConnection(targetID string) {
	h.pcMu.Lock()
	delete(h.pendingConnections, targetID)
	h.pcMu.Unlock()
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
				if !client.closed {
					client.closed = true
					close(client.Send)
				}
				log.Printf("[HUB] Client désenregistré: %s (Total: %d)", client.ID, len(h.clients))
			}
			h.mu.Unlock()
			h.ClearRelayPair(client.ID)

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
					if !client.closed {
						client.closed = true
						close(client.Send)
					}
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

// GetClientIDs retourne la liste des IDs des clients connectés
func (h *Hub) GetClientIDs() []string {
	h.mu.RLock()
	defer h.mu.RUnlock()
	ids := make([]string, 0, len(h.clients))
	for id := range h.clients {
		ids = append(ids, id)
	}
	return ids
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
			// Ne pas logger le contenu brut du message en production (risque de fuite de données chiffrées)
			log.Printf("[CLIENT %s] ❌ Erreur de parsing JSON (taille: %d bytes)", c.ID, len(message))
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

// checkRateLimit vérifie si le client dépasse la limite de messages
// Utilise une fenêtre glissante pour éviter le burst en début de fenêtre
func (c *Client) checkRateLimit() bool {
	c.rateLimitMu.Lock()
	defer c.rateLimitMu.Unlock()

	now := time.Now()
	elapsed := now.Sub(c.lastResetTime)

	// Reset le compteur si 1 minute est passée
	if elapsed >= time.Minute {
		c.messageCount = 0
		c.lastResetTime = now
	} else {
		// Fenêtre glissante : calcul proportionnel du budget restant
		// Si seulement 30s se sont écoulées, le budget est maxMessagesPerMin/2
		elapsedFraction := float64(elapsed) / float64(time.Minute)
		allowedSoFar := int(float64(c.maxMessagesPerMin) * elapsedFraction)
		if allowedSoFar < 10 {
			allowedSoFar = 10 // Minimum burst de 10 messages
		}
		if c.messageCount >= allowedSoFar {
			log.Printf("[RATE_LIMIT] Client %s dépasse la limite proportionnelle (%d/%d)", c.ID, c.messageCount, allowedSoFar)
			return false
		}
	}

	c.messageCount++
	if c.messageCount > c.maxMessagesPerMin {
		log.Printf("[RATE_LIMIT] ❌ Client %s dépasse la limite (%d messages/min)", c.ID, c.maxMessagesPerMin)
		return false
	}

	// Pas de log debug en mode normal — trop verbeux en production

	return true
}

// handleMessage traite un message reçu
func (c *Client) handleMessage(msg *models.Message) {
	// TypeRelay est haute-fréquence (30fps) — bypasse le rate limit, sécurisé par relay pair auth
	if msg.Type == models.TypeRelay {
		c.handleRelay(msg)
		return
	}

	// Vérifier le rate limit pour tous les autres messages
	if !c.checkRateLimit() {
		log.Printf("[CLIENT %s] ❌ Message rejeté (rate limit dépassé)", c.ID)
		return
	}

	// Log minimal — ne pas logguer le contenu des données (peuvent contenir des infos sensibles)
	log.Printf("[CLIENT %s] Message reçu: Type='%s'", c.ID, msg.Type)

	switch msg.Type {
	case models.TypeOffer:
		c.handleOffer(msg)
	case models.TypeAnswer:
		c.handleAnswer(msg)
	case models.TypeIceCandidate:
		c.handleIceCandidate(msg)
	case models.TypeConnectRequest:
		c.handleConnectRequest(msg)
	case models.TypeConnectionAccepted, models.TypeConnectionRejected:
		c.handleConnectionResponse(msg)
	case models.TypePasswordChallenge, models.TypePasswordResponse:
		c.handlePasswordMessage(msg)
	case models.TypePing:
		c.handlePing()
	default:
		log.Printf("[CLIENT] Type de message inconnu: %s", msg.Type)
	}
}

// handleOffer traite un message d'offre WebRTC
func (c *Client) handleOffer(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal offer: %v", err)
		return
	}

	// Validation : vérifier la taille des données (max MaxSDPSize pour offer)
	if len(data) > MaxSDPSize {
		log.Printf("[CLIENT %s] Offer SDP trop grande: %d bytes (max: %d)", c.ID, len(data), MaxSDPSize)
		c.sendAck("Offer", "error", "SDP trop grande")
		return
	}

	var offer models.OfferMessage
	if err := json.Unmarshal(data, &offer); err != nil {
		log.Printf("[CLIENT] Erreur parsing offer: %v", err)
		return
	}

	// Validation : vérifier que les champs obligatoires sont présents
	if offer.To == "" || offer.SDP == "" {
		log.Printf("[CLIENT] Offer invalide: champs obligatoires manquants")
		c.sendAck("Offer", "error", "Champs obligatoires manquants")
		return
	}

	// Transférer l'offre au destinataire
	c.Hub.SendToClient(offer.To, msg)

	// Envoyer ACK de succès à l'expéditeur
	c.sendAck("Offer", "success", "")
}

// handleAnswer traite un message de réponse WebRTC
func (c *Client) handleAnswer(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal answer: %v", err)
		return
	}

	// Validation : vérifier la taille (max MaxSDPSize pour answer)
	if len(data) > MaxSDPSize {
		log.Printf("[CLIENT %s] Answer SDP trop grande: %d bytes (max: %d)", c.ID, len(data), MaxSDPSize)
		c.sendAck("Answer", "error", "SDP trop grande")
		return
	}

	var answer models.AnswerMessage
	if err := json.Unmarshal(data, &answer); err != nil {
		log.Printf("[CLIENT] Erreur parsing answer: %v", err)
		return
	}

	// Validation : champs obligatoires
	if answer.To == "" || answer.SDP == "" {
		log.Printf("[CLIENT] Answer invalide: champs obligatoires manquants")
		c.sendAck("Answer", "error", "Champs obligatoires manquants")
		return
	}

	// Transférer la réponse au destinataire
	c.Hub.SendToClient(answer.To, msg)

	// Envoyer ACK de succès à l'expéditeur
	c.sendAck("Answer", "success", "")
}

// handleIceCandidate traite un candidat ICE
func (c *Client) handleIceCandidate(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal data: %v", err)
		c.sendAck("IceCandidate", "error", "Erreur de marshaling")
		return
	}

	// Validation : vérifier la taille (max MaxICESize pour ICE candidate)
	if len(data) > MaxICESize {
		log.Printf("[CLIENT %s] ICE candidate trop grande: %d bytes (max: %d)", c.ID, len(data), MaxICESize)
		c.sendAck("IceCandidate", "error", "ICE candidate trop grande")
		return
	}

	var ice models.IceCandidateMessage
	if err := json.Unmarshal(data, &ice); err != nil {
		log.Printf("[CLIENT] Erreur parsing ICE: %v", err)
		c.sendAck("IceCandidate", "error", "Erreur de parsing")
		return
	}

	// Valider que le destinataire est spécifié
	if ice.To == "" {
		log.Printf("[CLIENT] ICE candidate sans destinataire")
		c.sendAck("IceCandidate", "error", "Destinataire non spécifié")
		return
	}

	// Transférer le candidat au destinataire
	if err := c.Hub.SendToClient(ice.To, msg); err != nil {
		log.Printf("[CLIENT] Erreur envoi ICE à %s: %v", ice.To, err)
		c.sendAck("IceCandidate", "error", "Erreur d'envoi")
		return
	}

	// Envoyer ACK de succès à l'expéditeur
	c.sendAck("IceCandidate", "success", "")
}

// handleConnectRequest traite une demande de connexion
func (c *Client) handleConnectRequest(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal data: %v", err)
		c.sendAck("ConnectRequest", "error", "Erreur de marshaling")
		return
	}

	var req models.ConnectRequestMessage
	if err := json.Unmarshal(data, &req); err != nil {
		log.Printf("[CLIENT] Erreur parsing connect request (taille: %d bytes)", len(data))
		c.sendAck("ConnectRequest", "error", "Erreur de parsing")
		return
	}

	// Masquer le password dans les logs pour la sécurité
	passwordMasked := "***"
	if req.Password == nil {
		passwordMasked = "<none>"
	}
	log.Printf("[CLIENT %s] Demande de connexion vers %s (password: %s)", c.ID, req.TargetID, passwordMasked)

	// Rejeter l'auto-connexion
	if req.TargetID == c.ID {
		response := models.Message{
			Type: models.TypeError,
			Data: models.ErrorMessage{Code: 400, Message: "Impossible de se connecter à soi-même (même Device ID)"},
		}
		responseData, _ := json.Marshal(response)
		select {
		case c.Send <- responseData:
		default:
		}
		return
	}

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
				Message: "Appareil cible introuvable — vérifiez que l'autre PC est connecté au même serveur VPS",
			},
		}
		responseData, err := json.Marshal(response)
		if err != nil {
			log.Printf("[CLIENT] Erreur marshal response: %v", err)
			return
		}
		select {
		case c.Send <- responseData:
		default:
			log.Printf("[CLIENT %s] Canal saturé, message d'erreur non envoyé", c.ID)
		}

		// Envoyer ACK d'erreur
		c.sendAck("ConnectRequest", "error", "Client cible non trouvé")
		return
	}

	// Créer la notification de demande pour le client cible
	// On inclut l'ID de l'expéditeur pour que le client cible sache qui demande la connexion
	// NOTE: On envoie un flag has_password au lieu du password brut
	// Le client cible vérifie le password localement
	notification := models.Message{
		Type: models.TypeConnectRequest,
		Data: map[string]interface{}{
			"from":         c.ID,
			"has_password": req.Password != nil && *req.Password != "",
		},
	}

	// Enregistrer la paire pour validation des messages password
	c.Hub.RegisterPendingConnection(req.TargetID, c.ID)

	// Transférer la demande au client cible
	c.Hub.SendToClient(req.TargetID, notification)
	log.Printf("[HUB] Demande de connexion transférée de %s vers %s", c.ID, req.TargetID)

	// Envoyer ACK de succès à l'expéditeur
	c.sendAck("ConnectRequest", "success", "")
}

// handleConnectionResponse traite une réponse à une demande de connexion (acceptation ou rejet)
func (c *Client) handleConnectionResponse(msg *models.Message) {
	// Cette fonction sera appelée quand un client accepte ou rejette une connexion
	switch msg.Type {
	case models.TypeConnectionAccepted:
		data, err := json.Marshal(msg.Data)
		if err != nil {
			log.Printf("[CLIENT] Erreur marshal data: %v", err)
			return
		}
		var accepted models.ConnectionAcceptedMessage
		if err := json.Unmarshal(data, &accepted); err != nil {
			log.Printf("[CLIENT] Erreur parsing connection accepted: %v", err)
			return
		}
		log.Printf("[CLIENT %s] A accepté la connexion de %s", c.ID, accepted.PeerID)
		c.Hub.ClearPendingConnection(c.ID)
		c.Hub.SendToClient(accepted.PeerID, msg)
		// Enregistrer la paire relay dès l'acceptation (avant que les données commencent à circuler)
		c.Hub.RegisterRelayPair(c.ID, accepted.PeerID)

	case models.TypeConnectionRejected:
		data, err := json.Marshal(msg.Data)
		if err != nil {
			log.Printf("[CLIENT] Erreur marshal data: %v", err)
			return
		}
		var rejected models.ConnectionRejectedMessage
		if err := json.Unmarshal(data, &rejected); err != nil {
			log.Printf("[CLIENT] Erreur parsing connection rejected: %v", err)
			return
		}
		log.Printf("[CLIENT %s] A rejeté la connexion de %s: %s", c.ID, rejected.PeerID, rejected.Reason)
		c.Hub.ClearPendingConnection(c.ID)
		c.Hub.SendToClient(rejected.PeerID, msg)
	}
}

// handlePasswordMessage route les messages PasswordChallenge et PasswordResponse vers le pair
func (c *Client) handlePasswordMessage(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal password message: %v", err)
		return
	}

	// Extraire le peer_id du message pour le routage
	var peerMsg struct {
		PeerID string `json:"peer_id"`
	}
	if err := json.Unmarshal(data, &peerMsg); err != nil {
		log.Printf("[CLIENT] Erreur parsing password message: %v", err)
		return
	}

	if peerMsg.PeerID == "" {
		log.Printf("[CLIENT] Password message sans peer_id")
		return
	}

	// Vérifier que l'expéditeur fait partie de la paire de connexion autorisée
	if !c.Hub.IsPeerAuthorized(c.ID, peerMsg.PeerID) {
		log.Printf("[CLIENT %s] ❌ Tentative de routing %s non autorisée vers %s", c.ID, msg.Type, peerMsg.PeerID)
		return
	}

	log.Printf("[CLIENT %s] %s vers %s", c.ID, msg.Type, peerMsg.PeerID)
	c.Hub.SendToClient(peerMsg.PeerID, msg)
}

// handlePing traite un message de ping
func (c *Client) handlePing() {
	response := models.Message{
		Type: models.TypePong,
	}
	data, err := json.Marshal(response)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal pong: %v", err)
		return
	}
	select {
	case c.Send <- data:
	default:
		log.Printf("[CLIENT %s] Canal saturé, pong non envoyé", c.ID)
	}
}

// handleRelay relaie des données binaires (base64) entre deux pairs d'une session relay
func (c *Client) handleRelay(msg *models.Message) {
	data, err := json.Marshal(msg.Data)
	if err != nil {
		return
	}
	var relay models.RelayMessage
	if err := json.Unmarshal(data, &relay); err != nil {
		return
	}

	// Sécurité : le champ "from" doit correspondre à l'identité du client connecté
	if relay.From != c.ID {
		log.Printf("[CLIENT %s] ❌ Relay 'from' spoofé: %s", c.ID, relay.From)
		return
	}

	// Trouver le partenaire relay autorisé
	partner, ok := c.Hub.GetRelayPartner(c.ID)
	if !ok {
		log.Printf("[CLIENT %s] ❌ Relay sans partenaire enregistré", c.ID)
		return
	}

	// Transférer au partenaire (sans modifier le message)
	c.Hub.SendToClient(partner, msg)
}

// sendAck envoie un acquittement (ACK) au client
func (c *Client) sendAck(messageType string, status string, message string) {
	ackMsg := models.Message{
		Type: models.TypeAck,
		Data: models.AckMessage{
			MessageType: messageType,
			Status:      status,
			Message:     message,
		},
	}
	data, err := json.Marshal(ackMsg)
	if err != nil {
		log.Printf("[CLIENT] Erreur marshal ACK: %v", err)
		return
	}
	select {
	case c.Send <- data:
	default:
		log.Printf("[CLIENT %s] Canal saturé, ACK non envoyé", c.ID)
	}
}
