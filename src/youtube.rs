use rodio::{Decoder, OutputStreamHandle, Sink};
use serde_json::Value;
use std::{fs::File, io::BufReader};

use crate::{error::Error, NeoResult};

pub struct YoutubeClient {
    pub sink: Sink,
}

#[derive(Debug)]
pub struct YoutubeResult {
    pub title: String,
    pub href: String,
}

impl YoutubeClient {
    pub fn new(output_stream_handle: OutputStreamHandle) -> NeoResult<Self> {
        Ok(Self {
            sink: Sink::try_new(&output_stream_handle)
                .map_err(|e| Error::Other(format!("Rodio Error: {:?}", e)))?,
        })
    }

    pub fn search(query: String) -> NeoResult<Vec<YoutubeResult>> {
        let resp =
            ureq::get(format!("https://www.youtube.com/results?search_query={}", query).as_str())
                .call()?;

        serde_json::from_str::<Value>(
            resp.into_string()?
                .split("{\"itemSectionRenderer\":")
                .last()
                .ok_or_else(|| Error::Other(String::from("Split failed.")))?
                .split("},{\"continuationItemRenderer\":{")
                .collect::<Vec<&str>>()[0],
        )?
        .get("contents")
        .ok_or_else(|| Error::Other(String::from("Parsing Error: Can't find 'contents'.")))?
        .as_array()
        .ok_or_else(|| Error::Other(String::from("Parsing Error: Not an array.")))?
        .iter()
        .filter_map(|element| {
            if let Value::Object(obj) = element {
                obj.get("videoRenderer")
                    .map(|obj| -> NeoResult<YoutubeResult> {
                        Ok(YoutubeResult {
                            title: obj
                                .get("title")
                                .ok_or_else(|| {
                                    Error::Other(String::from("Parsing Error: Can't find 'title'."))
                                })?
                                .get("runs")
                                .ok_or_else(|| {
                                    Error::Other(String::from("Parsing Error: Can't find 'runs'."))
                                })?
                                .as_array()
                                .ok_or_else(|| {
                                    Error::Other(String::from("Parsing Error: Not an array."))
                                })?[0]
                                .get("text")
                                .ok_or_else(|| {
                                    Error::Other(String::from("Parsing Error: Can't find 'text'."))
                                })?
                                .to_string()
                                .replace('\"', ""),
                            href: obj
                                .get("videoId")
                                .ok_or_else(|| {
                                    Error::Other(String::from(
                                        "Parsing Error: Can't find 'videoId'.",
                                    ))
                                })?
                                .to_string()
                                .replace('\"', ""),
                        })
                    })
            } else {
                None
            }
        })
        .collect::<NeoResult<Vec<YoutubeResult>>>()
    }

    pub async fn play(&mut self, video_id: String) {
        let args = vec![
            ytd_rs::Arg::new("--quiet"),
            ytd_rs::Arg::new("-x"),
            ytd_rs::Arg::new_with_arg("--output", format!("{}.%(ext)s", video_id).as_str()),
            ytd_rs::Arg::new_with_arg("--audio-format", "mp3"),
        ];
        let path = std::path::PathBuf::from("./audio");
        let ytd = ytd_rs::YoutubeDL::new(
            &path,
            args,
            format!("https://www.youtube.com/watch?v={}", video_id).as_str(),
        )
        .unwrap();

        // start download
        ytd.download().unwrap();

        let file = BufReader::new(File::open(format!("./audio/{}.mp3", video_id)).unwrap());

        let source = Decoder::new(file).unwrap();

        self.sink.append(source);
        self.sink.play();
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn resume(&mut self) {
        self.sink.play();
    }
}
