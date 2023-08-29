use scraper::{Html, Selector};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::Markdown;

#[derive(strum_macros::Display, Debug, Clone, Copy, ToSchema, Deserialize)]
#[strum(serialize_all = "kebab-case")]
pub enum Building {
    Aastvej,
    Multihuset,
    Havremarken,
    #[strum(serialize = "kloeverblomsten-kirkbi")]
    KIRKBI,
    Midtown,
    Kornmarken,
    #[strum(serialize = "kantine-oestergade")]
    Oestergade,
}

pub(crate) async fn get_lunch(building: Building) -> anyhow::Result<Markdown> {
    let url = format!("https://lego.isscatering.dk/{}", building.to_string());
    let response = reqwest::get(url).await?.text().await?;
    let html = Html::parse_document(&response);
    let lunch = scrape_lunch(&html);
    let markdown = lunch_to_markdown(&lunch);

    Ok(markdown)
}

fn scrape_lunch(html: &Html) -> Lunch {
    let varm_ret_selector =
        Selector::parse("div.menu-row:nth-child(2) > div:nth-child(2)").unwrap();
    let vegetar_selector = Selector::parse("div.menu-row:nth-child(4) > div:nth-child(2)").unwrap();
    let salat_selector = Selector::parse("div.menu-row:nth-child(6) > div:nth-child(2)").unwrap();

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

#[derive(Debug, PartialEq)]
struct Lunch {
    varm_ret: String,
    vegetar: String,
    salat: String,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use scraper::Html;

    use crate::lunch;
    use crate::lunch::Lunch;

    #[test]
    fn lunch_to_markdown() {
        let lunch = Lunch {
            varm_ret: "Luftbøffer".to_owned(),
            vegetar: "Mælkebøtter".to_owned(),
            salat: "Gulerod".to_owned(),
        };

        let markdown = lunch::lunch_to_markdown(&lunch);
        let expected =
            "##### Varm ret\n  Luftbøffer\n##### Vegetar\n  Mælkebøtter\n##### Salat\n  Gulerod";
        assert_eq!(expected.to_owned(), markdown.0)
    }

    #[test]
    fn scrape_lunch() {
        let path_to_html =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/aastvej.html");
        let html_string = fs::read_to_string(path_to_html).unwrap();
        let html = Html::parse_document(&html_string);

        let result = lunch::scrape_lunch(&html);

        let expected = Lunch {
            varm_ret: "Braiseret svinekæber med rodfrugter".to_owned(),
            vegetar: "Gnocchi med ratatouille.".to_owned(),
            salat: "Romaine salat med bagte blommer, hvedekerner, løg og salatost.".to_owned(),
        };

        assert_eq!(result, expected);
    }
}
