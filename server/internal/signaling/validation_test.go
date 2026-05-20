package signaling

import (
	"strings"
	"testing"
)

func TestValidateDeviceID(t *testing.T) {
	tests := []struct {
		name      string
		deviceID  string
		shouldErr bool
		errMsg    string
	}{
		// Tests valides
		{"Valid GHD format", "GHD-abc123", false, ""},
		{"Valid alphanumeric", "Device123", false, ""},
		{"Valid with dashes", "my-device-01", false, ""},
		{"Minimum length 5 chars", "abcde", false, ""},
		{"Valid GHD minimum", "GHD-abcd", false, ""}, // GHD minimum = 8 chars
		{"Maximum length", strings.Repeat("a", 64), false, ""},

		// Tests invalides - Longueur
		{"Too short", "abcd", true, "trop court"},
		{"Too long", strings.Repeat("a", 65), true, "trop long"},

		// Tests invalides - Caractères
		{"With spaces", "GHD abc", true, "caractères invalides"},
		{"With special chars", "GHD@123", true, "caractères invalides"},
		{"With emojis", "GHD-😀", true, "caractères invalides"},
		{"With underscores", "GHD_123", true, "caractères invalides"},

		// Tests invalides - Format GHD
		{"GHD prefix too short", "GHD-ab", true, "format GHD invalide"},

		// Tests invalides - Edge cases
		{"Only dashes", "-----", true, "ne peut contenir que des tirets"},
		{"Multiple consecutive dashes", "GHD--123", true, "tirets consécutifs"},
		{"Empty string", "", true, "trop court"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateDeviceID(tt.deviceID)

			if tt.shouldErr {
				if err == nil {
					t.Errorf("Expected error for '%s', but got none", tt.deviceID)
				} else if !strings.Contains(strings.ToLower(err.Error()), strings.ToLower(tt.errMsg)) {
					t.Errorf("Expected error containing '%s', got: %v", tt.errMsg, err)
				}
			} else {
				if err != nil {
					t.Errorf("Unexpected error for '%s': %v", tt.deviceID, err)
				}
			}
		})
	}
}

func TestValidatePassword(t *testing.T) {
	tests := []struct {
		name      string
		password  string
		shouldErr bool
		errMsg    string
	}{
		// Tests valides
		{"Valid short password", "abc123", false, ""},
		{"Valid medium password", "mySecurePassword123!", false, ""},
		{"Valid long password", strings.Repeat("a", 128), false, ""},

		// Tests invalides
		{"Empty password", "", true, "vide"},
		{"Too long password", strings.Repeat("a", 129), true, "trop long"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validatePassword(tt.password)

			if tt.shouldErr {
				if err == nil {
					t.Errorf("Expected error for password (len=%d), but got none", len(tt.password))
				} else if !strings.Contains(strings.ToLower(err.Error()), strings.ToLower(tt.errMsg)) {
					t.Errorf("Expected error containing '%s', got: %v", tt.errMsg, err)
				}
			} else {
				if err != nil {
					t.Errorf("Unexpected error for password (len=%d): %v", len(tt.password), err)
				}
			}
		})
	}
}

func TestValidateTargetID(t *testing.T) {
	// validateTargetID devrait avoir les mêmes règles que validateDeviceID
	validIDs := []string{"GHD-abc123", "Device-01", "target123"}

	for _, id := range validIDs {
		if err := validateTargetID(id); err != nil {
			t.Errorf("validateTargetID failed for valid ID '%s': %v", id, err)
		}
	}

	invalidIDs := []string{"ab", strings.Repeat("a", 65), "test@123"}

	for _, id := range invalidIDs {
		if err := validateTargetID(id); err == nil {
			t.Errorf("validateTargetID should reject invalid ID '%s'", id)
		}
	}
}

// Benchmark pour vérifier que la validation n'impacte pas les performances
func BenchmarkValidateDeviceID(b *testing.B) {
	validID := "GHD-benchmark-test-123"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validateDeviceID(validID)
	}
}
