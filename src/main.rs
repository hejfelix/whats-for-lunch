use std::sync::Arc;

use tide::http::headers::LOCATION;
use tide::prelude::*;
use tide::StatusCode::MovedPermanently;
use tide::{http::Mime, Response};
use utoipa::OpenApi;
use utoipa_swagger_ui::Config;

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();
    let config = Arc::new(Config::from("/api-docs/openapi.json"));
    let mut app = tide::with_state(config);

    #[derive(OpenApi)]
    #[openapi(
        paths(
            lunch::get_lunch,
        ),
        components(
            schemas(lunch::Building)
        ),
        tags(
            (name = "lunch", description = "Lunch")
        )
    )]
    struct ApiDoc;

    // serve OpenApi json
    app.at("/api-docs/openapi.json")
        .get(|_| async move { Ok(Response::builder(200).body(json!(ApiDoc::openapi()))) });

    app.at("").get(|_| async {
        Ok(Response::builder(MovedPermanently)
            .header(LOCATION, "/swagger-ui/index.html")
            .build())
    });

    // serve Swagger UI
    app.at("/swagger-ui/*").get(serve_swagger);

    app.at("/api").nest({
        let mut todos = tide::new();
        todos.at("*/lunch").get(lunch::get_lunch);
        todos
    });

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}

async fn serve_swagger(request: tide::Request<Arc<Config<'_>>>) -> tide::Result<Response> {
    let config = request.state().clone();
    let path = request.url().path().to_string();
    let tail = path.strip_prefix("/swagger-ui/").unwrap();

    match utoipa_swagger_ui::serve(tail, config) {
        Ok(swagger_file) => swagger_file
            .map(|file| {
                Ok(Response::builder(200)
                    .body(file.bytes.to_vec())
                    .content_type(file.content_type.parse::<Mime>()?)
                    .build())
            })
            .unwrap_or_else(|| Ok(Response::builder(404).build())),
        Err(error) => Ok(Response::builder(500).body(error.to_string()).build()),
    }
}

mod lunch {
    use std::str::FromStr;
    use std::str::Split;

    use scraper::{Html, Selector};
    use serde::Serialize;
    use strum::EnumString;
    use tide::http::Url;
    use tide::prelude::*;
    use tide::{Error, Request, Response, StatusCode};
    use utoipa::ToSchema;

    #[derive(EnumString, Debug, Clone, Copy, ToSchema)]
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

    fn building_to_url(building: Building) -> Url {
        let path = match building {
            Building::Aastvej => "aastvej",
            Building::Multihuset => "multihuset",
            Building::Havremarken => "havremarken",
            Building::KIRKBI => "kloeverblomsten-kirkbi",
            Building::Midtown => "midtown",
            Building::Kornmarken => "kornmarken",
            Building::Oestergade => "kantine-oestergade",
        };
        let url_string = ["https://lego.isscatering.dk", path].join("/");
        Url::parse(&url_string).expect("url parsing")
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
    pub(super) async fn get_lunch(req: Request<()>) -> tide::Result<Response> {
        let mut path: Split<char> = req
            .url()
            .path_segments()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "path needed"))?;
        let building = path
            .next()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "path param missing"))?;

        let building_enum = Building::from_str(building)
            .ok()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "bad path param"))?;

        let building_url = building_to_url(building_enum);

        let response_body = surf::get(building_url).recv_string().await?;

        let html = Html::parse_document(&response_body);

        let lunch = scrape_lunch(&html);

        let markdown = lunch_to_markdown(lunch);

        let mattermost_response = MattermostCommandResponse {
            text: markdown,
            response_type: MattermostResponseType::InChannel,
        };

        Ok(Response::builder(StatusCode::Ok)
            .body(json!(mattermost_response))
            .build())
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

    fn lunch_to_markdown(lunch: Lunch) -> String {
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
        .join("")
    }

    #[derive(Debug)]
    struct Lunch {
        varm_ret: String,
        vegetar: String,
        salat: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(dead_code)] // Ephemeral not used currentlyg
    enum MattermostResponseType {
        InChannel,
        Ephemeral,
    }

    #[derive(Serialize)]
    struct MattermostCommandResponse {
        text: String,
        response_type: MattermostResponseType,
    }
}
