use rodio::{Decoder, OutputStreamHandle, Sink};
use serde_json::Value;
use std::{fs::File, io::BufReader};

pub struct YoutubeClient {
    pub sink: Sink,
}

#[derive(Debug)]
pub struct YoutubeResult {
    pub title: String,
    pub href: String,
}

impl YoutubeClient {
    pub fn new(output_stream_handle: OutputStreamHandle) -> Self {
        Self {
            sink: Sink::try_new(&output_stream_handle).unwrap(),
        }
    }

    pub fn search(query: String) -> Result<Vec<YoutubeResult>, ureq::Error> {
        let resp =
            ureq::get(format!("https://www.youtube.com/results?search_query={}", query).as_str())
                .call()?;

        Ok(serde_json::from_str::<Value>(
            resp.into_string()
                .unwrap()
                .split("{\"itemSectionRenderer\":")
                .last()
                .unwrap()
                .split("},{\"continuationItemRenderer\":{")
                .collect::<Vec<&str>>()[0],
        )
        .unwrap()
        .get("contents")
        .unwrap()
        .as_array()
        .unwrap()
        .into_iter()
        .filter_map(|element| {
            if let Value::Object(obj) = element {
                if let Some(obj) = obj.get("videoRenderer") {
                    Some(YoutubeResult {
                        title: obj
                            .get("title")
                            .unwrap()
                            .get("runs")
                            .unwrap()
                            .as_array()
                            .unwrap()[0]
                            .get("text")
                            .unwrap()
                            .to_string()
                            .replace("\"", ""),
                        href: obj.get("videoId").unwrap().to_string().replace("\"", ""),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<YoutubeResult>>())
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

    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end()
    }
}
