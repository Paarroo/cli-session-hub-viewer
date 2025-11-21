use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{self, Stream};
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;

/// SSE handler for real-time updates
pub async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| Event::default().event("heartbeat").data("ping"))
        .map(Ok)
        .throttle(Duration::from_secs(15));

    Sse::new(stream).keep_alive(KeepAlive::default())
}
