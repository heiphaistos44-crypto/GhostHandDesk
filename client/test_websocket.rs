use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080/ws";
    println!("[TEST] Connexion à {}...", url);

    match connect_async(url).await {
        Ok((ws_stream, response)) => {
            println!("[TEST] ✅ Connexion réussie !");
            println!("[TEST] Response: {:?}", response);
        }
        Err(e) => {
            eprintln!("[TEST] ❌ Erreur de connexion: {}", e);
            eprintln!("[TEST] Type d'erreur: {:?}", e);
        }
    }
}
