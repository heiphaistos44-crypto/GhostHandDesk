package signaling

import (
	"fmt"
	"regexp"
	"strings"
)

// validateDeviceID valide le format et la longueur d'un Device ID
// Format accepté: alphanumériques + tirets uniquement, longueur 5-64 caractères
// Si préfixe "GHD-" est présent, minimum 8 caractères (GHD-xxxx)
func validateDeviceID(id string) error {
	// Vérifier la longueur
	if len(id) < 5 {
		return fmt.Errorf("Device ID trop court: %d caractères (minimum: 5)", len(id))
	}

	if len(id) > 64 {
		return fmt.Errorf("Device ID trop long: %d caractères (maximum: 64)", len(id))
	}

	// Regex: alphanumériques + tiret uniquement (pas d'espaces, caractères spéciaux, emojis)
	matched, err := regexp.MatchString(`^[a-zA-Z0-9-]+$`, id)
	if err != nil {
		return fmt.Errorf("erreur de validation regex: %v", err)
	}

	if !matched {
		return fmt.Errorf("contient des caractères invalides (autorisés: a-z, A-Z, 0-9, -)")
	}

	// Validation spécifique pour format GHD-xxx
	if strings.HasPrefix(id, "GHD-") && len(id) < 8 {
		return fmt.Errorf("format GHD invalide (minimum: GHD-xxxx, 8 caractères)")
	}

	// Vérifier que le Device ID ne contient pas que des tirets
	if strings.Trim(id, "-") == "" {
		return fmt.Errorf("Device ID ne peut contenir que des tirets")
	}

	// Vérifier qu'il n'y a pas de tirets consécutifs multiples
	if strings.Contains(id, "--") {
		return fmt.Errorf("tirets consécutifs multiples non autorisés")
	}

	return nil
}

// validatePassword valide qu'un password n'est pas vide et a une longueur raisonnable
func validatePassword(password string) error {
	if password == "" {
		return fmt.Errorf("password vide")
	}

	if len(password) > 128 {
		return fmt.Errorf("password trop long: %d caractères (maximum: 128)", len(password))
	}

	return nil
}

// validateTargetID est un alias pour validateDeviceID (même règles)
func validateTargetID(targetID string) error {
	return validateDeviceID(targetID)
}
