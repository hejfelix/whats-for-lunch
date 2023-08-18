use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::get;
use axum::{Json, Router};
use log::info;
use lunch::Building;
use mattermost::MattermostCommandResponse;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

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

    info!("Listening on port 8080");
    
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

mod mattermost {
    use serde::Serialize;

    use crate::Markdown;

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(dead_code)] // Ephemeral not used currently
    enum MattermostResponseType {
        InChannel,
        Ephemeral,
    }

    #[derive(Serialize)]
    pub(crate) struct MattermostCommandResponse {
        text: String,
        response_type: MattermostResponseType,
    }

    impl MattermostCommandResponse {
        pub fn in_channel(markdown: Markdown) -> Self {
            Self {
                text: markdown.0,
                response_type: MattermostResponseType::InChannel,
            }
        }

        #[allow(dead_code)] // Ephemeral not used currently
        pub fn ephemeral(markdown: Markdown) -> Self {
            Self {
                text: markdown.0,
                response_type: MattermostResponseType::Ephemeral,
            }
        }
    }
}

mod lunch {
    use scraper::{Html, Selector};
    use serde::Deserialize;
    use strum::EnumString;
    use utoipa::ToSchema;

    use crate::Markdown;

    #[derive(EnumString, Debug, Clone, Copy, ToSchema, Deserialize)]
    pub enum Building {
        #[strum(ascii_case_insensitive)]
        Aastvej,
        #[strum(ascii_case_insensitive)]
        Multihuset,
        #[strum(ascii_case_insensitive)]
        Havremarken,
        #[strum(ascii_case_insensitive)]
        KIRKBI,
        #[strum(ascii_case_insensitive)]
        Midtown,
        #[strum(ascii_case_insensitive)]
        Kornmarken,
        #[strum(ascii_case_insensitive)]
        Oestergade,
    }

    fn building_to_url(building: &Building) -> String {
        let path = match building {
            Building::Aastvej => "aastvej",
            Building::Multihuset => "multihuset",
            Building::Havremarken => "havremarken",
            Building::KIRKBI => "kloeverblomsten-kirkbi",
            Building::Midtown => "midtown",
            Building::Kornmarken => "kornmarken",
            Building::Oestergade => "kantine-oestergade",
        };
        format!("https://lego.isscatering.dk/{path}")
    }

    pub(crate) async fn get_lunch(building: Building) -> anyhow::Result<Markdown> {
        let url = building_to_url(&building);
        let response = reqwest::get(url).await?.text().await?;
        let html = Html::parse_document(&response);
        let lunch = scrape_lunch(&html);
        let markdown = lunch_to_markdown(&lunch);

        Ok(markdown)
    }

    fn scrape_lunch(html: &Html) -> Lunch {
        let varm_ret_selector =
            Selector::parse("div.menu-row:nth-child(2) > div:nth-child(2)").unwrap();
        let vegetar_selector =
            Selector::parse("div.menu-row:nth-child(4) > div:nth-child(2)").unwrap();
        let salat_selector =
            Selector::parse("div.menu-row:nth-child(6) > div:nth-child(2)").unwrap();

        let varm_ret = html
            .select(&varm_ret_selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        let vegetar = html
            .select(&vegetar_selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        let salat = html
            .select(&salat_selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        Lunch {
            varm_ret: String::from(varm_ret.trim()),
            vegetar: String::from(vegetar.trim()),
            salat: String::from(salat.trim()),
        }
    }

    fn lunch_to_markdown(lunch: &Lunch) -> Markdown {
        Markdown(
            [
                "##### Varm ret\n  ",
                lunch.varm_ret.as_str(),
                "\n",
                "##### Vegetar\n  ",
                lunch.vegetar.as_str(),
                "\n",
                "##### Salat\n  ",
                lunch.salat.as_str(),
            ]
            .join(""),
        )
    }

    #[derive(Debug)]
    struct Lunch {
        varm_ret: String,
        vegetar: String,
        salat: String,
    }
}
