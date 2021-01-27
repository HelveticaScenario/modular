use warp::Filter;

#[tokio::main]
async fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));
    let js = warp::path("js").and(warp::fs::dir("./modular_web/client/dist/"));
    warp::serve(index.or(js).or(hello))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

static INDEX_HTML: &str = include_str!("../client/index.html");
