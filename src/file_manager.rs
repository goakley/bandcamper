use std::ffi::OsStr;

use crate::types::*;

lazy_static::lazy_static! {
    static ref RE_FORBIDDEN: regex::Regex = regex::Regex::new(r#"[/<>:|?*"\\]"#).unwrap();
}

pub struct FileManager {
    pub root_directory: std::path::PathBuf,
}

impl FileManager {
    fn get_album_directory(&self, artist: &str, title: &str) -> std::path::PathBuf {
        let cartist = RE_FORBIDDEN.replace_all(artist, "_");
        let mut path = self.root_directory.join(cartist.into_owned());
        let ctitle = RE_FORBIDDEN.replace_all(title, "_");
        path.push(ctitle.into_owned());
        path
    }

    fn get_track_directory(&self, artist: &str) -> std::path::PathBuf {
        let cartist = RE_FORBIDDEN.replace_all(artist, "_");
        self.root_directory.join(cartist.into_owned())
    }

    pub fn is_completed(&self, collection_item: &CollectionItem) -> Result<bool, std::io::Error> {
        match collection_item.itype {
            CollectionItemKind::Album => {
                let dir = self.get_album_directory(&collection_item.artist, &collection_item.title);
                if !dir.exists() {
                    return Ok(false);
                }
                if dir.join(".incomplete").exists() {
                    return Ok(false);
                }
                Ok(dir.read_dir()?.next().is_some())
            }
            CollectionItemKind::Track => Ok(false),
        }
    }

    pub fn is_completed_file(
        &self,
        collection_item: &CollectionItem,
        filename: &str,
    ) -> Result<bool, std::io::Error> {
        match collection_item.itype {
            CollectionItemKind::Album => self.is_completed(collection_item),
            CollectionItemKind::Track => {
                let dir = self
                    .get_track_directory(&collection_item.artist)
                    .join(filename);
                Ok(dir.exists())
            }
        }
    }

    pub fn complete(&self, collection_item: &CollectionItem) -> Result<(), std::io::Error> {
        let mut dir = match collection_item.itype {
            CollectionItemKind::Album => {
                self.get_album_directory(&collection_item.artist, &collection_item.title)
            }
            CollectionItemKind::Track => self.get_track_directory(&collection_item.artist),
        };
        dir.push(".incomplete");
        if dir.exists() {
            std::fs::remove_file(dir)?;
        }
        Ok(())
    }

    pub fn get_filepath(
        &self,
        collection_item: &CollectionItem,
        filename: &std::path::Path,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), std::io::Error> {
        let dir = match collection_item.itype {
            CollectionItemKind::Album => {
                self.get_album_directory(&collection_item.artist, &collection_item.title)
            }
            CollectionItemKind::Track => self.get_track_directory(&collection_item.artist),
        };
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        std::fs::File::create(dir.join(".incomplete"))?;
        let tempstr: &OsStr = ".temporary.".as_ref();
        let mut tempstring = tempstr.to_os_string();
        tempstring.push(filename);
        Ok((dir.join(tempstring), dir.join(filename)))
    }
}
