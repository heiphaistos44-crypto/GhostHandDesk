package models

import "time"

// MessageType représente le type de message de signalement
type MessageType string

const (
	// Types de messages
	TypeRegister           MessageType = "Register"
	TypeOffer              MessageType = "Offer"
	TypeAnswer             MessageType = "Answer"
	TypeIceCandidate       MessageType = "IceCandidate"
	TypeConnectRequest     MessageType = "ConnectRequest"
	TypeConnectionAccepted MessageType = "ConnectionAccepted"
	TypeConnectionRejected MessageType = "ConnectionRejected"
	TypePing               MessageType = "Ping"
	TypePong               MessageType = "Pong"
	TypeError              MessageType = "Error"
	TypeAck                MessageType = "Ack" // Acquittement de réception
)

// Message représente un message de signalement
type Message struct {
	Type MessageType `json:"type"`
	Data interface{} `json:"data,omitempty"`
}

// RegisterMessage - message d'enregistrement d'un dispositif
type RegisterMessage struct {
	DeviceID string `json:"device_id"`
}

// OfferMessage - message d'offre WebRTC
type OfferMessage struct {
	From string `json:"from"`
	To   string `json:"to"`
	SDP  string `json:"sdp"`
}

// AnswerMessage - message de réponse WebRTC
type AnswerMessage struct {
	From string `json:"from"`
	To   string `json:"to"`
	SDP  string `json:"sdp"`
}

// IceCandidateMessage - message de candidat ICE
type IceCandidateMessage struct {
	From          string `json:"from"`
	To            string `json:"to"`
	Candidate     string `json:"candidate"`
	SDPMid        string `json:"sdp_mid"`
	SDPMLineIndex uint16 `json:"sdp_mline_index"`
}

// ConnectRequestMessage - demande de connexion
type ConnectRequestMessage struct {
	TargetID string  `json:"target_id"`
	Password *string `json:"password,omitempty"`
}

// ConnectionAcceptedMessage - connexion acceptée
type ConnectionAcceptedMessage struct {
	PeerID string `json:"peer_id"`
}

// ConnectionRejectedMessage - connexion rejetée
type ConnectionRejectedMessage struct {
	PeerID string `json:"peer_id"`
	Reason string `json:"reason"`
}

// ErrorMessage - message d'erreur
type ErrorMessage struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

// AckMessage - acquittement de réception d'un message
type AckMessage struct {
	MessageType string `json:"message_type"` // Type du message acquitté
	Status      string `json:"status"`       // "success" ou "error"
	Message     string `json:"message,omitempty"`
}

// Client représente un client connecté
type Client struct {
	ID             string
	Connection     interface{} // *websocket.Conn (évite l'import circulaire)
	LastSeen       time.Time
	ConnectedPeers map[string]bool
}
