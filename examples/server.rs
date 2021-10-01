use axum::{
    response::{sse::Event, Sse},
    Router,
};
use futures_core::Stream;
use rand_core::{OsRng, RngCore};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let router = Router::new().route("/", axum::handler::get(event_stream));
    let arg = std::env::args().nth(1);
    let listen_uri = arg.as_deref().unwrap_or("0.0.0.0:3000");

    println!("Starting server on {}", listen_uri);

    axum::Server::bind(&listen_uri.parse().expect("URI"))
        .serve(router.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn event_stream() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(EventStream::default())
}

#[derive(Default)]
pub struct EventStream {
    counter: u64,
}

impl Stream for EventStream {
    type Item = Result<Event, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let event = match OsRng.next_u32() % 4 {
            0 => Event::default().data("DataEvent"),
            1 => Event::default().comment("Comment, please ignore!"),
            2 => Event::default().event("EventField"),
            3 => Event::default().retry(std::time::Duration::from_secs(1)),
            _ => unreachable!(),
        };
        self.counter += 1;
        let event = event.id(self.counter.to_string());
        println!("Returning Event: {:?}", event);
        Poll::Ready(Some(Ok(event)))
    }
}
