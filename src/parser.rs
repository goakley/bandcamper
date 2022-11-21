use json_dotpath::DotPaths;
use serde::Deserialize;

use crate::types::{CollectionItem, CollectionItemKind, Encoding};

lazy_static::lazy_static! {
    static ref RE_MB: regex::Regex = regex::Regex::new(r#"^([0-9.]+)([GgMmKk])[Bb]?$"#).unwrap();
    static ref SE_COL: scraper::Selector = scraper::Selector::parse("li#collection-main > a").unwrap();
    static ref SE_DIV_PAGEDATA: scraper::Selector = scraper::Selector::parse("div#pagedata").unwrap();
}

#[derive(Debug)]
pub enum ParsePageError {
    NoHtmlElement(&'static scraper::Selector),
    PageDataNotFound,
    PageDataNotJSON(serde_json::Error),
    InvalidJSON(&'static str, json_dotpath::Error),
    UnexpectedDownloadDigitalItemCount(usize),
}

impl From<serde_json::Error> for ParsePageError {
    fn from(err: serde_json::Error) -> Self {
        ParsePageError::PageDataNotJSON(err)
    }
}

fn parse_data_blob(scraper: &scraper::html::Html) -> Result<serde_json::Value, ParsePageError> {
    let div = scraper
        .select(&SE_DIV_PAGEDATA)
        .next()
        .ok_or(ParsePageError::PageDataNotFound)?;
    let blob = div
        .value()
        .attr("data-blob")
        .ok_or(ParsePageError::PageDataNotFound)?;
    let value = serde_json::from_str(blob)?;
    Ok(value)
}

macro_rules! page_data_dot_get {
    ($a:expr, $b:expr) => {
        $b.dot_get($a)
            .map_err(|e| ParsePageError::InvalidJSON($a, e))?
            .ok_or(ParsePageError::InvalidJSON(
                $a,
                json_dotpath::Error::BadPathElement,
            ))?
    };
}

pub fn parse_home_page(html: &str) -> Result<(String, String), ParsePageError> {
    let html = scraper::Html::parse_document(html);
    let collection_a = html
        .select(&SE_COL)
        .next()
        .ok_or(ParsePageError::NoHtmlElement(&SE_COL))?;
    let href = match collection_a.value().attr("href") {
        Some(attr) => Ok(attr.to_string()),
        None => Err(ParsePageError::NoHtmlElement(&SE_COL)),
    }?;
    let value = parse_data_blob(&html)?;
    let username: String = page_data_dot_get!("identities.fan.username", value);
    Ok((username, href))
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ItemCacheCollectionItem {
    #[serde(alias = "item_title")]
    title: String,
    #[serde(alias = "band_name")]
    artist: String,
    #[serde(alias = "tralbum_type")]
    tralbum_type: String,
    #[serde(alias = "tralbum_id")]
    tralbum_id: u64,
    #[serde(alias = "item_type")]
    item_type: CollectionItemKind,
    #[serde(alias = "item_id")]
    item_id: u64,
    #[serde(alias = "sale_item_id")]
    sale_item_id: u64,
    #[serde(alias = "sale_item_type")]
    sale_item_type: String,
}

#[derive(Debug)]
pub struct CollectionPageData {
    pub fan_id: u64,
    pub last_token: String,
    pub collection_items: Vec<CollectionItem>,
}

pub fn parse_collection_page(html: &str) -> Result<CollectionPageData, ParsePageError> {
    let html = scraper::Html::parse_document(html);
    let value = parse_data_blob(&html)?;
    let mut collection: std::collections::HashMap<String, ItemCacheCollectionItem> =
        page_data_dot_get!("item_cache.collection", value);
    let mut redownload_urls: std::collections::HashMap<String, String> =
        page_data_dot_get!("collection_data.redownload_urls", value);
    //let download_urls: std::collections::HashMap<u64, String> = redownload_urls.drain().map(|(mut k, v)| (k.drain(1..).collect::<String>().parse().unwrap(), v)).collect();
    let sequence: Vec<String> = page_data_dot_get!("collection_data.sequence", value);
    let fan_id: u64 = page_data_dot_get!("fan_data.fan_id", value);
    let last_token: String = page_data_dot_get!("collection_data.last_token", value);
    let collection_items: Vec<CollectionItem> = sequence
        .iter()
        .map(|seq| {
            let item = collection.remove(seq).unwrap();
            let url = redownload_urls
                .remove(&format!("{}{}", item.sale_item_type, item.sale_item_id))
                .unwrap();
            CollectionItem {
                itype: item.item_type,
                title: item.title,
                artist: item.artist,
                download_url: url,
            }
        })
        .collect();
    Ok(CollectionPageData {
        fan_id,
        last_token,
        collection_items,
    })
}

#[derive(Deserialize)]
struct CollectionJSONObject {
    items: Vec<ItemCacheCollectionItem>,
    last_token: String,
    more_available: bool,
    redownload_urls: std::collections::HashMap<String, String>,
}

pub struct CollectionJSON {
    pub more_available: bool,
    pub last_token: String,
    pub collection_items: Vec<CollectionItem>,
}

pub fn parse_collection_json(data: &str) -> Result<CollectionJSON, serde_json::Error> {
    let mut collection: CollectionJSONObject = serde_json::from_str(data)?;
    let collection_items: Vec<CollectionItem> = collection
        .items
        .into_iter()
        .map(|item| {
            let url = collection
                .redownload_urls
                .remove(&format!("{}{}", item.sale_item_type, item.sale_item_id))
                .unwrap();
            CollectionItem {
                itype: item.item_type,
                title: item.title,
                artist: item.artist,
                download_url: url,
            }
        })
        .collect();
    Ok(CollectionJSON {
        more_available: collection.more_available,
        last_token: collection.last_token,
        collection_items,
    })
}

fn deserialize_megabytes<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    let caps = RE_MB
        .captures(&text)
        .ok_or_else(|| serde::de::Error::custom("invalid megabyte text"))?;
    let number: f64 = caps[1]
        .parse()
        .map_err(|e| serde::de::Error::custom(format!("bad megabyte number: {:?}", e)))?;
    let chara = caps[2].chars().next().and_then(|c| c.to_lowercase().next());
    match chara {
        Some('k') => Ok((number * 1024f64) as u64),
        Some('m') => Ok((number * 1024f64 * 1024f64) as u64),
        Some('g') => Ok((number * 1024f64 * 1024f64 * 1024f64) as u64),
        x => Err(serde::de::Error::custom(format!(
            "bad megabyte number unit: {:?}",
            x
        ))),
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadOption {
    //description: String,
    #[serde(alias = "encoding_name")]
    pub encoding: Encoding,
    #[serde(alias = "size_mb", deserialize_with = "deserialize_megabytes")]
    pub approximate_size: u64,
    pub url: String,
}

pub fn parse_download_page(html: &str) -> Result<std::vec::Vec<DownloadOption>, ParsePageError> {
    let html = scraper::Html::parse_document(html);
    let value = parse_data_blob(&html)?;
    let digital_items: std::vec::Vec<serde_json::Value> =
        page_data_dot_get!("digital_items", value);
    if digital_items.len() != 1 {
        return Err(ParsePageError::UnexpectedDownloadDigitalItemCount(
            digital_items.len(),
        ));
    }
    let downloads: std::collections::HashMap<String, DownloadOption> =
        page_data_dot_get!("digital_items.0.downloads", value);
    Ok(downloads.into_values().collect())
}

#[cfg(test)]
mod tests {
    use crate::parser::deserialize_megabytes;
    use serde::Deserialize;
    use serde_test::{assert_de_tokens, Token};

    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct MBTest {
        #[serde(deserialize_with = "deserialize_megabytes")]
        size: u64,
    }

    #[test]
    fn test_deserialize_megabytes() {
        let tokens = vec![
            (1181116006u64, "1.1GB"),
            (1179648000, "1125MB"),
            (11808768, "11532kb"),
            (11809382, "11532.6K"),
        ];
        for (size, token) in tokens.into_iter() {
            assert_de_tokens(
                &MBTest { size: size },
                &[
                    Token::Struct {
                        name: "MBTest",
                        len: 1,
                    },
                    Token::Str("size"),
                    Token::Str(token),
                    Token::StructEnd,
                ],
            );
        }
    }
}
