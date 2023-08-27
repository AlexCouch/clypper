use curl::easy::Easy;
use indicatif::{ProgressStyle, ProgressBar};
use regex::Regex;
use console::style;

///
/// This struct holds the start and end times in milliseconds of the clip 
///
#[derive(Clone, Copy, Debug, Default)]
pub struct ClipTime(pub u64, pub u64);

#[derive(Clone, Debug, Default)]
pub struct ClipResource(pub String, pub String);

#[derive(Clone, Debug, Default)]
pub struct Clip<'url>{
    pub url: &'url str,
    pub resource: ClipResource,
    pub time: ClipTime,
}

#[derive(Clone, Debug)]
pub enum ClypperError{
    ///Message, Error code?
    CurlError(String, usize),
    ///args: message, pattern
    RegexError(String, String),
    FFmpegError,
}

pub struct Extractor{
    video_url_re: Regex,
    audio_url_re: Regex,
    timestamp_re: Regex,

    spinner: ProgressBar,
}

impl Extractor{
    fn get_html(&self, url: &str) -> Result<String, ClypperError>{
        let mut easy = Easy::new();
        easy.url(url).unwrap();
        let mut buffer = Vec::new();
        
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    buffer.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();
            transfer.perform().unwrap();
        }
        let buffer = buffer.clone();
        let html = String::from_utf8(buffer).unwrap();
        Ok(html)
    }

    pub fn new() -> Result<Self, ClypperError>{
        let style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}").unwrap();
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(style);

        Ok(Self{
            video_url_re: Regex::new(r#"\"itag\":\d+,\"url\":\"(.+?)\".+?\"width\":(\d+)"#).unwrap(),
            audio_url_re: Regex::new(r#"itag\":\d+,\"url\":\"([^\"\s]*)\",\"mimeType\":\"audio/mp4;"#).unwrap(),
            timestamp_re: Regex::new(r#"\"clipConfig\":\{\"postId\":\".+\",\"startTimeMs\":\"(\d+?)\",\"endTimeMs\":\"(\d+?)\""#).unwrap(), 
            spinner,
        })
    }

    pub fn extract<'a>(&'a self, url: &'a str) -> Result<Clip, ClypperError>{
        self.spinner.set_message("Getting clip info...");
        let html_result = self.get_html(url).unwrap();
        let html = html_result.as_str();
        let mut video_url = String::new();
        //TODO: Add more proper error handling
        for (_, [url, width]) in self.video_url_re.captures_iter(html).map(|c| c.extract()){
            if width == "1920"{
                video_url = String::from(url);
            }
        };
        //TODO: Add more proper error handling
        let audio_url = self.audio_url_re.captures(html).unwrap().get(1).unwrap();
        let escape = Regex::new(r#"\\u0026"#).unwrap();
        let vid_url = String::from(escape.replace_all(video_url.as_str(), "&"));
        let aud_url = String::from(escape.replace_all(audio_url.as_str(), "&"));

        let timestamp_match = self.timestamp_re.captures(html).unwrap();
        let start_ms: u64 = timestamp_match.get(1).unwrap().as_str().parse().unwrap();
        let end_ms: u64 = timestamp_match.get(2).unwrap().as_str().parse().unwrap();
        self.spinner.finish_with_message("Getting clip info... Done!");
        Ok(Clip { url, resource: ClipResource(vid_url, aud_url), time: ClipTime(start_ms, end_ms) })
    }
}

