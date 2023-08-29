use axum::{Json, Router};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::get;
use log::info;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

use lunch::Building;
use mattermost::MattermostCommandResponse;

mod lunch;
mod mattermost;


#[derive(OpenApi)]
#[openapi(
    paths(
        get_lunch,
    ),
    components(
        schemas(lunch::Building)
    ),
    tags(
        (name = "lunch", description = "Lunch")
    )
)]
struct ApiDoc;

pub(crate) struct Markdown(String);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let api = Router::new().route("/:building/lunch", get(get_lunch));

    let app = Router::new()
        .merge(RapiDoc::with_openapi("/api-docs/openapi.json", ApiDoc::openapi()).path("/rapidoc"))
        .route("/", get(|| async { Redirect::permanent("/rapidoc") }))
        .nest("/api", api)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    info!("Listening on http://127.0.0.1:8080");

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[utoipa::path(
    get,
    path = "/api/{building}/lunch",
    params(
        ("building" = Building, Path, description = "the building for which to get lunch")
    ),
    responses(
        (status = 200, description = "Get lunch for specified building")
    )
)]
async fn get_lunch(
    Path(building): Path<Building>,
) -> Result<Json<MattermostCommandResponse>, StatusCode> {
    match lunch::get_lunch(building).await {
        Ok(markdown_lunch) => Ok(Json(MattermostCommandResponse::in_channel(markdown_lunch))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}