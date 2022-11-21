use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
pub enum Encoding {
    #[serde(rename = "aac-hi")]
    Aac, // aac-hi
    #[serde(rename = "aiff-lossless")]
    Aiff, // aiff-lossless
    #[serde(rename = "alac")]
    Alac, // alac
    #[serde(rename = "flac")]
    Flac, // flac
    #[serde(rename = "mp3-320")]
    MP3320, // mp3-320
    #[serde(rename = "mp3-v0")]
    MP3V0, // mp3-v0
    #[serde(rename = "vorbis")]
    Ogg, // vorbis
    #[serde(rename = "wav")]
    Wav, // wav
}

impl fmt::Display for Encoding {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Encoding::Aac => write!(f, "aac"),
            Encoding::Aiff => write!(f, "aiff"),
            Encoding::Alac => write!(f, "alac"),
            Encoding::Flac => write!(f, "flac"),
            Encoding::MP3320 => write!(f, "mp3_320"),
            Encoding::MP3V0 => write!(f, "mp3_v0"),
            Encoding::Ogg => write!(f, "ogg"),
            Encoding::Wav => write!(f, "wav"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum CollectionItemKind {
    #[serde(alias = "album")]
    Album,
    #[serde(alias = "track")]
    Track,
}

#[derive(Debug)]
pub struct CollectionItem {
    pub itype: CollectionItemKind,
    pub title: String,
    pub artist: String,
    pub download_url: String,
}
