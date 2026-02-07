use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

fn main() {
    let msg = SignalMessage {
        msg_type: "ConnectRequest".to_string(),
        data: Some(serde_json::json!({
            "target_id": "GHD-test123",
            "password": Option::<String>::None
        })),
    };

    let json = serde_json::to_string_pretty(&msg).unwrap();
    println!("Message sérialisé :");
    println!("{}", json);
}
