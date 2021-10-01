use futures_util::StreamExt;
use sse_agent::Sse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let arg = std::env::args().nth(1);
    let target_uri = arg.as_deref().unwrap_or("http://0.0.0.0:3000");

    let response = reqwest::get(target_uri).await?;

    let mut sse = response.bytes_stream().into_sse();

    while let Some(Ok(ev)) = sse.next().await {
        println!("{:?}", ev);
    }

    Ok(())
}
