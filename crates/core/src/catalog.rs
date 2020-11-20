use crate::config::Flavor;
use crate::network::request_async;
use crate::Result;
use chrono::prelude::*;

use isahc::{config::RedirectPolicy, prelude::*};
use serde::Deserialize;

const CURSE_CATALOG_URL: &str =
    "https://github.com/casperstorm/ajour-catalog/releases/latest/download/curse.json";
const TUKUI_CATALOG_URL: &str =
    "https://github.com/casperstorm/ajour-catalog/releases/latest/download/tukui.json";
const WOWI_CATALOG_URL: &str =
    "https://github.com/casperstorm/ajour-catalog/releases/latest/download/wowi.json";

pub async fn get_catalog() -> Result<Catalog> {
    let client = HttpClient::builder()
        .redirect_policy(RedirectPolicy::Follow)
        .max_connections_per_host(6)
        .build()
        .unwrap();

    let mut curse_resp = request_async(&client, CURSE_CATALOG_URL, vec![], Some(30)).await?;
    let mut tukui_resp = request_async(&client, TUKUI_CATALOG_URL, vec![], Some(30)).await?;
    let mut wowi_resp = request_async(&client, WOWI_CATALOG_URL, vec![], Some(30)).await?;

    let mut addons = vec![];
    if curse_resp.status().is_success() {
        let mut catalog: Catalog = curse_resp.json()?;
        addons.append(&mut catalog.addons);
    }

    if tukui_resp.status().is_success() {
        let mut catalog: Catalog = tukui_resp.json()?;
        addons.append(&mut catalog.addons);
    }

    if wowi_resp.status().is_success() {
        let mut catalog: Catalog = wowi_resp.json()?;
        addons.append(&mut catalog.addons);
    }

    Ok(Catalog { addons })
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    #[serde(alias = "curse")]
    Curse,
    #[serde(alias = "tukui")]
    Tukui,
    #[serde(alias = "wowi")]
    WowI,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Source::Curse => "Curse",
            Source::Tukui => "Tukui",
            Source::WowI => "WowInterface",
        };
        write!(f, "{}", s)
    }
}

#[serde(transparent)]
#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub addons: Vec<CatalogAddon>,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct GameVersion {
    pub game_version: String,
    pub flavor: Flavor,
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogAddon {
    pub id: i32,
    pub website_url: String,
    #[serde(with = "date_parser")]
    pub date_released: Option<DateTime<Utc>>,
    pub name: String,
    pub categories: Vec<String>,
    pub summary: String,
    pub number_of_downloads: u64,
    pub source: Source,
    #[deprecated(since = "0.4.4", note = "Please use game_versions instead")]
    pub flavors: Vec<Flavor>,
    pub game_versions: Vec<GameVersion>,
}

mod date_parser {
    use chrono::prelude::*;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Curse format
        let date = DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&Utc))
            .ok();

        if date.is_some() {
            return Ok(date);
        }

        // Tukui format
        let date = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %T")
            .map(|d| Utc.from_utc_datetime(&d))
            .ok();

        if date.is_some() {
            return Ok(date);
        }

        // Handles Elvui and Tukui addons which runs in a format without HH:mm:ss.
        let s_modified = format!("{} 00:00:00", &s);
        let date = NaiveDateTime::parse_from_str(&s_modified, "%Y-%m-%d %T")
            .map(|d| Utc.from_utc_datetime(&d))
            .ok();

        if date.is_some() {
            return Ok(date);
        }

        // Handles WowI.
        if let Ok(ts) = &s.parse::<i64>() {
            let date = Utc.timestamp(ts / 1000, 0);
            return Ok(Some(date));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_download() {
        async_std::task::block_on(async {
            let catalog = get_catalog().await;

            if let Err(e) = catalog {
                panic!("{}", e);
            }
        });
    }
}
