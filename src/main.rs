use atrium_api::{agent::atp_agent::{store::MemorySessionStore, AtpAgent}};
use atrium_xrpc_client::reqwest::ReqwestClient;
use config::Config;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

use atrium_api::types::{TryFromUnknown, Unknown};

#[derive(Deserialize)]
struct ConfigFie {
    bluesky_identifier: String,
    bluesky_password: String,
}
#[derive(Debug, serde::Deserialize)]
struct Mention {
    text: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();
    let config: ConfigFie = settings.try_deserialize().unwrap();

    let client = AtpAgent::new(
        ReqwestClient::new("https://bsky.social"),
        MemorySessionStore::default(),
    );
    client.login(config.bluesky_identifier, config.bluesky_password).await?;
    let session = client
        .api
        .com
        .atproto
        .server
        .get_session()
        .await?;

    // Create a session using the provided credentials
    println!("Login successful. DID: {:?}", session.did);

    // Main loop to check for mentions every 5 seconds
    loop {
        match check_mentions(&client).await {
            Ok(()) => println!("Checked for mentions successfully"),
            Err(e) => eprintln!("Error checking mentions: {}", e),
        }
        
        // Wait 5 seconds before next check
        sleep(Duration::from_secs(5)).await;
    }
}

async fn check_mentions(client: &AtpAgent<MemorySessionStore, ReqwestClient>) -> Result<(), Box<dyn std::error::Error>> {
    // Get notifications (which include mentions)
    let mentions = client.api
        .app
        .bsky
        .notification
        .list_notifications(
            atrium_api::app::bsky::notification::list_notifications::ParametersData {
                cursor: None,
                reasons: Some(vec!["mention".to_string()]),
                limit: Some(50.try_into().unwrap()),
                priority: None,
                seen_at: None,
            }
            .into(),
        )
        .await?;
    
    println!("Found {} mentions", mentions.data.notifications.len());

    for mut mention in mentions.data.notifications {
        
        println!("Mention from {:?}", mention.author.handle);
        println!("Reason: {}", mention.reason);
        println!("Record URI: {}", mention.uri);

        let record = Mention::try_from_unknown(mention.record.clone())?;
        println!("{:?}", record.text);

        mention.is_read = false; // Mark as unread for demonstration purposes
    }  
    Ok(())
}