use clap::Parser;
use serde::Serialize;
use std::io::Write;

mod downloader;
mod file_manager;
mod parser;
mod types;

use crate::downloader::*;
use crate::file_manager::*;
use crate::parser::*;
use crate::types::*;

#[derive(Parser)]
struct Args {
    #[arg(short='f', long="format", default_value_t = Encoding::Flac)]
    format: Encoding,
    #[arg(short = 'u', long = "username")]
    username: Option<String>,

    path: std::ffi::OsString,
}

fn pick_format<'a>(
    preferences: &std::vec::Vec<Encoding>,
    items: &'a std::vec::Vec<DownloadOption>,
) -> Option<&'a DownloadOption> {
    for preference in preferences {
        for item in items {
            if &item.encoding == preference {
                return Some(item);
            }
        }
    }
    None
}

#[derive(Debug)]
enum HandleDownloadResponseError {
    BadZip(zip::result::ZipError),
    BadIO(std::io::Error),
}

impl From<std::io::Error> for HandleDownloadResponseError {
    fn from(err: std::io::Error) -> Self {
        HandleDownloadResponseError::BadIO(err)
    }
}

fn handle_download_response<D: MediaDownload>(
    file_manager: &FileManager,
    item: &CollectionItem,
    media_download: &mut D,
) -> Result<(), HandleDownloadResponseError> {
    if file_manager
        .is_completed_file(item, media_download.get_filename())
        .unwrap()
    {
        println!("  Skipping download (already completed)");
        return Ok(());
    }
    if media_download.get_filename().ends_with(".zip") {
        //let tempfile = file_manager.get_tempfilepath(item).unwrap();
        let filename: std::path::PathBuf = media_download.get_filename().to_string().into();
        let (ziptemp, _) = file_manager.get_filepath(item, &filename).unwrap();
        let mut tmp = std::fs::File::create(&ziptemp)?;
        media_download.save(&mut tmp).unwrap();
        tmp.flush()?;
        let file = std::fs::File::open(&ziptemp).map_err(HandleDownloadResponseError::BadIO)?;
        let mut ziparchive =
            zip::ZipArchive::new(file).map_err(HandleDownloadResponseError::BadZip)?;
        for i in 0..ziparchive.len() {
            let mut file = ziparchive
                .by_index(i)
                .map_err(HandleDownloadResponseError::BadZip)?;
            if !file.is_file() {
                continue;
            }
            let name = file
                .enclosed_name()
                .ok_or(HandleDownloadResponseError::BadZip(
                    zip::result::ZipError::UnsupportedArchive("invalid filename in archive"),
                ))?;
            let (tempfile, realfile) = file_manager.get_filepath(item, name)?;
            if (realfile).exists() {
                std::fs::remove_file(&realfile)?;
            }
            let mut fsfile = std::fs::File::create(&tempfile)?;
            std::io::copy(&mut file, &mut fsfile)?;
            std::fs::rename(tempfile, realfile)?;
        }
        std::fs::remove_file(ziptemp)?;
    } else {
        let mut filepath = std::path::PathBuf::new();
        filepath.set_file_name(media_download.get_filename());
        let (tempfile, realfile) = file_manager.get_filepath(item, &filepath)?;
        if (realfile).exists() {
            std::fs::remove_file(&realfile)?;
        }
        let mut tmp = std::fs::File::create(&tempfile).unwrap();
        media_download.save(&mut tmp).unwrap();
        tmp.flush().unwrap();
        // yes, we remove twice
        if (realfile).exists() {
            std::fs::remove_file(&realfile)?;
        }
        std::fs::rename(tempfile, &realfile)?;
    }
    file_manager.complete(item)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct CollectionItemsRequestBody {
    fan_id: u64,
    older_than_token: String,
    count: u64,
}

fn load_bandcamp_cookies() -> Vec<(String, String, Downloader)> {
    let cookies_results: Vec<bench_scraper::KnownBrowserCookies> =
        bench_scraper::find_cookies().unwrap();
    let mut bandcamp_cookies: Vec<Vec<bench_scraper::Cookie>> = cookies_results
        .into_iter()
        .map(|kbcookies| {
            kbcookies
                .cookies
                .into_iter()
                .filter(|c| c.host.ends_with("bandcamp.com"))
                .collect()
        })
        .filter(|v: &Vec<bench_scraper::Cookie>| !v.is_empty())
        .collect();
    bandcamp_cookies.sort_by_key(|is| is.iter().map(|i| i.last_accessed).max());
    bandcamp_cookies
        .into_iter()
        .filter_map(|cookies| {
            let header_values: Vec<reqwest::header::HeaderValue> = cookies
                .iter()
                .map(|i| {
                    let cookie_string = format!("{}={}; Domain=bandcamp.com", i.name, i.value);
                    reqwest::header::HeaderValue::from_str(&cookie_string).unwrap()
                })
                .collect();
            let downloader = Downloader::new(header_values).unwrap();
            let home_page = downloader
                .get_page("https://bandcamp.com")
                .unwrap()
                .text()
                .unwrap();
            parse_home_page(&home_page)
                .ok()
                .map(|(a, b)| (a, b, downloader))
        })
        .collect()
}

fn get_collection_link(username: Option<String>) -> Option<(String, Downloader)> {
    let cookies = load_bandcamp_cookies();
    match (username, cookies.as_slice()) {
        (_, []) => {
            println!("You are not logged into Bandcamp through your web browser.");
            println!("Please log in at https://bandcamp.com/login, and then try running this program again.");
            None
        }
        (Some(usr), [(u, c, d)]) => {
            if &usr == u {
                Some((c.to_string(), d.clone()))
            } else {
                println!("You are logged into the Bandcamp account of `{}.`", u);
                println!(
                    "I can't fetch the collection of `{}` with that login information.",
                    usr
                );
                println!("Please re-login to Bandcamp as `{}` or run the program with `--username '{}'` to download their collection instead.", usr, u);
                None
            }
        }
        (None, [(_, c, d)]) => Some((c.to_string(), d.clone())),
        (None, xs) => {
            println!("You are logged into multiple Bandcamp accounts in multiple browsers.");
            println!("I don't know which account you want to download music from.");
            println!("Please specify the `--username` flag so that I know what to download.");
            let examples: Vec<String> = xs
                .iter()
                .map(|(u, _, _)| format!("`--username {}`", u))
                .collect();
            println!("e.g: {}", examples.join(" or "));
            None
        }
        (Some(usr), xs) => match xs.iter().find(|(u, _, _)| &usr == u) {
            None => {
                println!(
                    "You are not logged into Bandcamp as `{}` in any browser.",
                    usr
                );
                println!("Please log in at https://bandcamp.com/login, and then try running this program again.");
                None
            }
            Some((_, c, d)) => Some((c.to_string(), d.clone())),
        },
    }
}

fn main() {
    println!("Loading configuration settings");
    let settings = Args::parse();
    let format_preferences = vec![settings.format];
    let file_manager = FileManager {
        root_directory: settings.path.into(),
    };
    if !file_manager.root_directory.exists() {
        std::fs::create_dir(&file_manager.root_directory).unwrap();
    }
    let (collection_link, downloader) = get_collection_link(settings.username).unwrap();
    let collection_page = downloader
        .get_page(&collection_link)
        .unwrap()
        .text()
        .unwrap();
    let collection_page_data = parse_collection_page(&collection_page).unwrap();
    let fan_id = collection_page_data.fan_id;
    let mut all_collection_items: Vec<CollectionItem> = collection_page_data.collection_items;
    let mut older_than_token = collection_page_data.last_token;
    let mut more_available = true;
    while more_available {
        let body = CollectionItemsRequestBody {
            fan_id,
            older_than_token,
            count: 20,
        };
        let collection_json = downloader
            .post_api(
                "https://bandcamp.com/api/fancollection/1/collection_items",
                &body,
            )
            .unwrap()
            .text()
            .unwrap();
        let mut collection_data = parse_collection_json(&collection_json).unwrap();
        all_collection_items.append(&mut collection_data.collection_items);
        older_than_token = collection_data.last_token;
        more_available = collection_data.more_available;
    }
    for item in all_collection_items.iter() {
        println!(
            "Processing item: {:?} \"{}\" by \"{}\"",
            item.itype, item.title, item.artist
        );
        if file_manager.is_completed(item).unwrap() {
            println!("  Item already processed");
            continue;
        }
        println!("  Analysing download page {:?}", &item.download_url);
        let contents = downloader
            .get_page(&item.download_url)
            .unwrap()
            .text()
            .unwrap();
        // std::fs::File::create("/tmp/item.html")
        //     .unwrap()
        //     .write_all(contents.as_bytes())
        //     .unwrap();
        let download_options = parse_download_page(&contents).unwrap();
        let download_option = pick_format(&format_preferences, &download_options).unwrap();
        println!(
            "  Downloading data (~{} bytes) {:?}",
            download_option.approximate_size, download_option.url,
        );
        let mut media_download = downloader.get_media(&download_option.url).unwrap();
        handle_download_response(&file_manager, item, &mut media_download).unwrap();
        println!("  Finished processing item");
    }
}

#[cfg(test)]
mod tests {
    use crate::downloader::*;
    use crate::file_manager::*;
    use crate::handle_download_response;
    use crate::types::*;

    struct MockMediaDownload {
        filename: String,
    }
    impl MediaDownload for MockMediaDownload {
        fn get_filename(&self) -> &str {
            &self.filename
        }
        fn save<W: ?Sized + std::io::Write>(
            &mut self,
            writer: &mut W,
        ) -> Result<(), reqwest::Error> {
            let mut source =
                std::fs::File::open(std::path::Path::new("testdata").join(&self.filename)).unwrap();
            let xxx = std::io::copy(&mut source, writer).unwrap();
            println!("{:?}", xxx);
            Ok(())
        }
    }
    struct MockBadMediaDownload {
        filename: String,
    }
    impl MediaDownload for MockBadMediaDownload {
        fn get_filename(&self) -> &str {
            &self.filename
        }
        fn save<W: ?Sized + std::io::Write>(
            &mut self,
            _writer: &mut W,
        ) -> Result<(), reqwest::Error> {
            panic!("BAD");
        }
    }

    #[test]
    fn download_response_unzip() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
        ));
        std::fs::create_dir(&dir).unwrap();
        let file_manager = FileManager {
            root_directory: dir.clone(),
        };
        let item = CollectionItem {
            itype: CollectionItemKind::Album,
            title: "Abc 123".to_string(),
            artist: "My CR".to_string(),
            download_url: "".to_string(),
        };
        let mut media_download = MockMediaDownload {
            filename: "archive.zip".to_string(),
        };
        handle_download_response(&file_manager, &item, &mut media_download).unwrap();
        let mut missing: std::collections::HashSet<std::ffi::OsString> =
            std::collections::HashSet::new();
        missing.insert("file1.flac".into());
        missing.insert("file2.flac".into());
        for entry in dir.join("My CR").join("Abc 123").read_dir().unwrap() {
            let name = entry.unwrap().file_name();
            assert!(missing.remove(&name));
        }
        assert!(missing.is_empty());
        let mut bad_media_download = MockBadMediaDownload {
            filename: "track.flac".to_string(),
        };
        handle_download_response(&file_manager, &item, &mut bad_media_download).unwrap();
    }

    #[test]
    fn download_response_track() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
        ));
        std::fs::create_dir(&dir).unwrap();
        let file_manager = FileManager {
            root_directory: dir.clone(),
        };
        let mut media_download = MockMediaDownload {
            filename: "track.flac".to_string(),
        };
        let item = CollectionItem {
            itype: CollectionItemKind::Track,
            title: "Hewwo".to_string(),
            artist: "Boopers".to_string(),
            download_url: "".to_string(),
        };
        handle_download_response(&file_manager, &item, &mut media_download).unwrap();
        let mut missing: std::collections::HashSet<std::ffi::OsString> =
            std::collections::HashSet::new();
        missing.insert("track.flac".into());
        for entry in dir.join("Boopers").read_dir().unwrap() {
            let name = entry.unwrap().file_name();
            assert!(missing.remove(&name));
        }
        assert!(missing.is_empty());
        let mut bad_media_download = MockBadMediaDownload {
            filename: "track.flac".to_string(),
        };
        handle_download_response(&file_manager, &item, &mut bad_media_download).unwrap();
    }
}
