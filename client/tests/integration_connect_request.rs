// Test d'intégration pour valider que ConnectRequest parvient au serveur
use ghost_hand_client::network::SignalMessage;
use serde_json;

#[test]
fn test_connect_request_serialization() {
    // Test 1: ConnectRequest SANS password
    let msg_without_pwd = SignalMessage::connect_request(
        "GHD-target123".to_string(),
        None
    );

    let json_str = serde_json::to_string(&msg_without_pwd).unwrap();
    println!("ConnectRequest sans password: {}", json_str);

    // Vérifier que le JSON contient "type": "ConnectRequest"
    assert!(json_str.contains(r#""type":"ConnectRequest"#));
    assert!(json_str.contains(r#""target_id":"GHD-target123"#));

    // Test 2: ConnectRequest AVEC password
    let msg_with_pwd = SignalMessage::connect_request(
        "GHD-target456".to_string(),
        Some("password123".to_string())
    );

    let json_str = serde_json::to_string(&msg_with_pwd).unwrap();
    println!("ConnectRequest avec password: {}", json_str);

    assert!(json_str.contains(r#""type":"ConnectRequest"#));
    assert!(json_str.contains(r#""target_id":"GHD-target456"#));
    assert!(json_str.contains(r#""password":"password123"#));
}

#[test]
fn test_connect_request_deserialization() {
    // Simuler ce que le serveur Go reçoit
    let json_from_server = r#"{
        "type": "ConnectRequest",
        "data": {
            "target_id": "GHD-abc123",
            "password": "test123"
        }
    }"#;

    let msg: SignalMessage = serde_json::from_str(json_from_server).unwrap();
    assert_eq!(msg.msg_type, "ConnectRequest");

    let data = msg.data.as_ref().unwrap();
    assert_eq!(data["target_id"], "GHD-abc123");
    assert_eq!(data["password"], "test123");
}

#[test]
fn test_connect_request_format_compatibility() {
    // Vérifier que le format Rust est 100% compatible avec le format Go attendu
    let msg = SignalMessage::connect_request(
        "GHD-test".to_string(),
        Some("pwd".to_string())
    );

    let json_value: serde_json::Value = serde_json::to_value(&msg).unwrap();

    // Vérifier la structure exacte
    assert_eq!(json_value["type"], "ConnectRequest");
    assert!(json_value["data"].is_object());
    assert_eq!(json_value["data"]["target_id"], "GHD-test");
    assert_eq!(json_value["data"]["password"], "pwd");

    // Vérifier qu'il n'y a pas de champs supplémentaires
    let data_obj = json_value["data"].as_object().unwrap();
    assert_eq!(data_obj.len(), 2); // Seulement target_id et password
}

#[test]
fn test_connect_request_message_structure() {
    // Vérifier que la construction du message ne crash pas
    let msg = SignalMessage::connect_request(
        "GHD-target".to_string(),
        Some("password".to_string())
    );

    assert_eq!(msg.msg_type, "ConnectRequest");

    // Vérifier que les données sont présentes
    assert!(msg.data.is_some());

    let data = msg.data.unwrap();
    assert_eq!(data["target_id"], "GHD-target");
    assert_eq!(data["password"], "password");

    println!("✅ Test complet: Message construit avec succès");
}
