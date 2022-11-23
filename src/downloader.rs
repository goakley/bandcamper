use reqwest::cookie::CookieStore;
use serde::Serialize;

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::new(10, 0);
const MEDIA_TIMEOUT: std::time::Duration = std::time::Duration::new(30 * 60, 0);

lazy_static::lazy_static! {
    static ref RE_CD: regex::bytes::Regex = regex::bytes::Regex::new(r#"filename\s*=\s*"([^"]+)""#).unwrap();
}

pub trait MediaDownload {
    fn get_filename(&self) -> &str;
    fn save<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> Result<(), reqwest::Error>;
}

pub struct ReqwestMediaDownload {
    content_length: u64,
    filename: String,
    response: reqwest::blocking::Response,
}

impl MediaDownload for ReqwestMediaDownload {
    fn get_filename(&self) -> &str {
        &self.filename
    }

    fn save<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> Result<(), reqwest::Error> {
        let count = self.response.copy_to(writer)?;
        assert!(count == self.content_length);
        Ok(())
    }
}

#[derive(Debug)]
pub enum GetMediaError {
    RequestError(reqwest::Error),
    NoContentDisposition(String),
    InvalidContentDisposition(Vec<u8>),
    NoContentLength,
    InvalidContentLength(Vec<u8>),
    InvalidFilename(Vec<u8>, std::str::Utf8Error),
}

#[derive(Clone)]
pub struct Downloader {
    client: reqwest::blocking::Client,
}

impl Downloader {
    pub fn new(cookies: Vec<reqwest::header::HeaderValue>) -> reqwest::Result<Self> {
        let url = "https://bandcamp.com".parse::<reqwest::Url>().unwrap();
        let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
        jar.set_cookies(&mut cookies.iter(), &url);
        let builder = reqwest::blocking::Client::builder()
            .user_agent("bandcamper")
            .gzip(true)
            .cookie_store(true)
            .cookie_provider(jar)
            .timeout(DEFAULT_TIMEOUT);
        let client = builder.build()?;
        Ok(Downloader { client })
    }

    pub fn get_page(&self, url: &str) -> reqwest::Result<reqwest::blocking::Response> {
        self.client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
    }

    pub fn post_api<T: Serialize + ?Sized>(
        &self,
        url: &str,
        json: &T,
    ) -> reqwest::Result<reqwest::blocking::Response> {
        self.client
            .post(url)
            .json(json)
            .send()
            .and_then(|r| r.error_for_status())
    }

    pub fn get_media(&self, url: &str) -> Result<ReqwestMediaDownload, GetMediaError> {
        let response = self
            .client
            .get(url)
            .timeout(MEDIA_TIMEOUT)
            .send()
            .map_err(GetMediaError::RequestError)?
            .error_for_status()
            .map_err(GetMediaError::RequestError)?;
        let content_disposition_x = response.headers().get(reqwest::header::CONTENT_DISPOSITION);
        let content_disposition = match content_disposition_x {
            Some(cd) => Ok(cd),
            _ => {
                // let mut fff = std::fs::File::create("/tmp/error.html").unwrap();
                // response.copy_to(&mut fff).unwrap();
                Err(GetMediaError::NoContentDisposition(format!(
                    "{:?}",
                    &response.headers()
                )))
            }
        }?;
        let capture = RE_CD
            .captures(content_disposition.as_bytes())
            .ok_or_else(|| {
                GetMediaError::InvalidContentDisposition(content_disposition.as_bytes().into())
            })?;
        let filename = std::str::from_utf8(&capture[1])
            .map_err(|e| GetMediaError::InvalidFilename(capture[1].into(), e))?
            .to_string();
        let content_length_h = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .ok_or(GetMediaError::NoContentLength)?;
        let content_length: u64 = content_length_h
            .to_str()
            .map_err(|_| GetMediaError::InvalidContentLength(content_length_h.as_bytes().into()))?
            .parse()
            .map_err(|_| GetMediaError::InvalidContentLength(content_length_h.as_bytes().into()))?;
        Ok(ReqwestMediaDownload {
            content_length,
            filename,
            response,
        })
    }
}
