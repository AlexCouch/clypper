use std::{
    io::{BufRead, BufReader},
    process::{self, Command, Stdio},
    thread,
};

use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

use crate::extract::extractor::{Clip, ClypperError};
use numtoa::NumToA;

use chrono::{naive::NaiveTime, NaiveDateTime};

type FFmpegHandle = process::Child;

pub struct Downloader<'dl> {
    clip: Clip<'dl>,
    out: String,

    ffmpeg_handle: Option<FFmpegHandle>,

    progress_bar: ProgressBar,
}

impl<'dl> Downloader<'dl> {
    pub fn new(clip: Clip<'dl>, out: String) -> Self {
        let pb = ProgressBar::new(clip.time.1);
        pb.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/grey} {percent}% {msg}").unwrap());
        Self {
            clip: clip.clone(),
            out,
            ffmpeg_handle: None,
            progress_bar: pb,
        }
    }

    pub fn download(&mut self) -> Result<(), ClypperError> {
        let mut buffer = [0u8; 20];
        let start = format!("{}ms", self.clip.time.0.numtoa_str(10, &mut buffer));
        let mut buffer2 = [0u8; 20];
        let end = format!("{}ms", self.clip.time.1.numtoa_str(10, &mut buffer2));

        let mut command = Command::new("ffmpeg");
        command
            .args(["-ss", start.as_str()])
            .args(["-to", end.as_str()])
            .args(["-i", self.clip.resource.0.as_str()])
            .args(["-ss", start.as_str()])
            .args(["-to", end.as_str()])
            .args(["-i", self.clip.resource.1.as_str()])
            .args(["-map", "0:v"])
            .args(["-map", "1:a"])
            .args(["-c:v", "libx264"])
            .args(["-c:a", "aac"])
            .arg("pipe:1")
            .arg("-y")
            .arg(self.out.as_str());
        command.stderr(Stdio::piped()).stdout(Stdio::piped());
        let handle = match command.spawn() {
            Ok(child) => child,
            Err(_) => return Err(ClypperError::FFmpegError),
        };

        let end_time = NaiveDateTime::from_timestamp_millis(self.clip.time.1 as i64).unwrap().time();
        let out = handle.stdout.unwrap();
        let pb = self.progress_bar.clone();
        pb.set_message("Loading ffmpeg...");
        pb.inc(0);
        //let stdout_thread = thread::spawn(move ||{
            let time_re = Regex::new(r#"time=(\d+:\d+:\d+\.\d+)"#).unwrap();
            let buf = BufReader::new(out);
            for line in buf.lines(){
                let line = line.unwrap();
                let ln_str = line.as_str();
                let capture = if let Some(capture) = time_re.captures(ln_str){
                    capture
                }else{
                    continue
                };
                pb.set_message("Downloading clip...");
                let time = capture.get(1).unwrap().as_str();
                let naive_time = NaiveTime::parse_from_str(time, "%H:%M:%S.%f").unwrap();
                let signed_delta = NaiveTime::signed_duration_since(naive_time, end_time).num_milliseconds();
                let delta: u64 = if signed_delta < 0{
                    (signed_delta * -1) as u64
                } else {
                    signed_delta as u64
                };
                pb.inc(delta);
            }
        //});
        //stdout_thread.join().expect("Failed to join stdout thread");
        Ok(())
    }
}
