use atrium_api::{client, models::{Message, Mention}};
use reqwest::Client as HttpClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use config::Config;
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
struct XPost {
    text: String,
    attachments: Vec<XAttachment>,
}

#[derive(Deserialize)]
struct XAttachment {
    url: String,
}

#[derive(Deserialize)]
struct ThreadsPost {
    content: String,
    media: Vec<ThreadsMedia>,
}

#[derive(Deserialize)]
struct ThreadsMedia {
    media_url: String,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    bluesky_api_key: String,
    bluesky_api_secret: String,
    threads_api_key: String,
    twitter_api_key: String,
}

#[tokio::main]
async fn main() {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();
    let config: AppConfig = settings.try_deserialize().unwrap();

    let bluesky_client = Client::new(config.bluesky_api_key, config.bluesky_api_secret);
    let http_client = HttpClient::new();

    loop {
        let mentions = bluesky_client.get_mentions().await.unwrap();

        for mention in mentions {
            if let Some(message) = process_mention(&mention).await {
                if let Some(link) = extract_link(&message.text) {
                    if link.contains("x.com") {
                        if let Ok(post_content) = fetch_x_post(&http_client, &link).await {
                            post_to_bluesky(&bluesky_client, post_content.text, post_content.attachments).await.unwrap();
                        }
                    } else if link.contains("threads.net") || link.contains("threads.com") {
                        if let Ok(post_content) = fetch_threads_post(&http_client, &link).await {
                            post_to_bluesky(&bluesky_client, post_content.content, post_content.media).await.unwrap();
                        }
                    }
                }
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}

async fn process_mention(mention: &Mention) -> Option<Message> {
    if mention.text.contains("@copypost") {
        return Some(mention.clone());
    }
    None
}

fn extract_link(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"https?://[^\s]+").unwrap();
    re.find(text).map(|m| m.as_str().to_string())
}

async fn fetch_x_post(client: &HttpClient, url: &str) -> Result<XPost, reqwest::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str("Bearer {}", config.twitter_api_key).unwrap());

    let post_id = extract_post_id(url)?;
    let response = client.get(format!("https://api.x.com/2/tweets/{}", post_id)) // Adjust endpoint as per API documentation
        .headers(headers)
        .send()
        .await?;
    let post: XPost = response.json().await?;
    Ok(post)
}

async fn fetch_threads_post(client: &HttpClient, url: &str) -> Result<ThreadsPost, reqwest::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str("Bearer {}", config.threads_api_key).unwrap());

    let post_id = extract_post_id(url)?;
    let response = client.get(format!("https://graph.threads.net/v1/posts/{}", post_id)) // Adjust endpoint as per API documentation
        .headers(headers)
        .send()
        .await?;
    let post: ThreadsPost = response.json().await?;
    Ok(post)
}

fn extract_post_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?<=/posts/)\w+").unwrap();
    re.find(url).map(|m| m.as_str().to_string())
}

async fn post_to_bluesky(client: &Client, content: String, media: Vec<ThreadsMedia>) -> Result<(), reqwest::Error> {
    let mut media_ids = Vec::new();

    // Upload media and collect media IDs
    for media_item in media {
        let media_id = upload_media_to_bluesky(client, media_item.media_url).await?;
        media_ids.push(media_id);
    }

    // Create a post with the media IDs
    client.create_post(content, media_ids).await.unwrap(); // Adjust based on the actual post creation method
    Ok(())
}

async fn upload_media_to_bluesky(client: &Client, media_url: String) -> Result<String, reqwest::Error> {
    let media_response = reqwest::get(&media_url).await?;
    let media_bytes = media_response.bytes().await?;

    // Upload media to Bluesky
    let media_upload_response = client.upload_media(media_bytes).await.unwrap(); // Adjust based on the actual upload method

    // Assuming the response contains a media ID
    let media_id = media_upload_response.media_id; // Adjust this line based on the actual response structure
    Ok(media_id)
}