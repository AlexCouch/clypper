use std::{
    fmt::Display,
    io::{BufRead, BufReader, Stdout},
    process::{self, Command, Stdio},
    sync::{Arc, Mutex},
    thread, ffi::OsStr, borrow::Cow,
};

use regex::Regex;

use crate::extract::extractor::{Clip, ClipResource, ClipTime};

type FFmpegHandle = process::Child;
type FFmpegThread = std::thread::JoinHandle<Result<(), FFmpegError>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FFmpegState {
    #[default]
    NotStarted,
    Starting,
    Downloading(u64),
    Finished,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct FFmpegError {
    pub message: String,
    pub state: FFmpegState,
}

#[derive(Debug, Default)]
pub struct FFmpegInput<'input> {
    url: &'input str,
    start_ms: &'input str,
    end_ms: &'input str,
}

impl<'input> FFmpegInput<'input> {
    pub fn new(url: &'input str, start_ms: &'input str, end_ms: &'input str) -> Self {
        Self {
            url,
            start_ms,
            end_ms,
        }
    }
}

pub struct FFmpeg<'ffmpeg, OnProgressCallback, OnStateChangeCallback> {
    process: Option<FFmpegHandle>,
    inputs: Vec<Cow<'ffmpeg, str>>,
    start_ms: u64,
    start_ms_str: Cow<'ffmpeg, str>,
    end_ms: u64,
    end_ms_str: Cow<'ffmpeg, str>,
    output: Option<&'ffmpeg str>,

    state: FFmpegState,
    ffmpeg_thread: Option<FFmpegThread>,
    errors: Arc<Mutex<Vec<FFmpegError>>>,

    on_progress_callback: Option<Box<OnProgressCallback>>,
    on_state_change_callback: Option<Box<OnStateChangeCallback>>,
}

impl<'ffmpeg, OnProgressCallback, OnStateChangeCallback> FFmpeg<'ffmpeg, OnProgressCallback, OnStateChangeCallback>
where
    OnProgressCallback: Fn(u64) -> (),
    OnStateChangeCallback: Fn(FFmpegState) -> (),
{
    pub fn new() -> Self{
        Self{
            process: None,
            inputs: vec![],
            start_ms: 0,
            start_ms_str: "".into(),
            end_ms: 0,
            end_ms_str: "".into(),
            output: None,

            state: FFmpegState::default(),
            ffmpeg_thread: None,
            errors: Arc::default(),

            on_progress_callback: None,
            on_state_change_callback: None,
        }
    }

}

impl<'ffmpeg, OnProgressCallback, OnStateChangeCallback> FFmpeg<'ffmpeg, OnProgressCallback, OnStateChangeCallback>
where
    OnProgressCallback: Fn(u64) -> (),
    OnStateChangeCallback: Fn(FFmpegState) -> (),
{
    pub fn state_change_callback(&mut self, callback: OnStateChangeCallback) -> &mut Self{
        self.on_state_change_callback = Some(Box::new(callback));
        self
    }

    pub fn progress_callback(&mut self, callback: OnProgressCallback) -> &mut Self {
        self.on_progress_callback = Some(Box::new(callback));
        self
    }
    pub fn output(&mut self, output: &'ffmpeg str) -> Result<&mut Self, FFmpegError> {
        self.output = Some(output);

        Ok(self)
    }

    pub fn time(
        &mut self,
        start_ms: u64,
        end_ms: u64
    ) -> Result<&mut Self, FFmpegError>{
        self.start_ms = start_ms;
        let start_str = format!("{}ms", start_ms);
        self.start_ms_str = start_str.into();
        self.end_ms = end_ms;
        self.end_ms_str = format!("{}ms", end_ms).into();

        Ok(self)
    }
    pub fn input(
        &mut self,
        url: &'ffmpeg str,
    ) -> Result<&mut Self, FFmpegError> {
        self.inputs.push(url.into());

        Ok(self)
    }

    fn change_state(&mut self, state: FFmpegState){
        self.state = state;
        if let Some(ref state_change_callback) = self.on_state_change_callback{
            state_change_callback(self.state.clone());
        }
    }

    pub fn spawn(&mut self) -> Result<(), FFmpegError> {
        let mut command = Command::new("ffmpeg");
        for input in self.inputs.clone(){
            command.args([
                "-ss", self.start_ms_str.as_ref(), 
                "-to", self.end_ms_str.as_ref(), 
                "-i", input.to_string().as_str()
            ]);
        }
        command
            .args(["-hide_banner", "-progress", "pipe:2", "-y", "-map", "0:v", "-map", "1:a",  "-c:v", "libx264", "-c:a", "aac"])
            .args(self.output.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        //for arg in command.get_args(){
        //    println!("{:?}", arg);
        //}
        self.change_state(FFmpegState::Starting);

        let process = command.spawn().unwrap();
        let stderr = process.stderr.ok_or_else(|| FFmpegError {
            message: "Failed to get stderr from ffmpeg process".to_string(),
            state: self.state.clone(),
        })?;
        {
            let errors = self.errors.clone();
            let buffer = BufReader::new(stderr);
            let (ffmpeg_send, ffmpeg_recv) = std::sync::mpsc::channel();
            self.ffmpeg_thread = Some(std::thread::spawn(move || {
                let re = Regex::new(r#"\btime=(\d+):(\d+):(\d+)\.(\d+)"#).unwrap();
                buffer
                    .lines()
                    .filter_map(|line| line.ok())
                    .for_each(|line| {
                        let cap = re
                            .captures(line.as_str())
                            .map(|capture| {
                                let hr = capture.get(1).unwrap().as_str();
                                let min = capture.get(2).unwrap().as_str();
                                let sec = capture.get(3).unwrap().as_str();
                                let ms = capture.get(4).unwrap().as_str();
                                (hr, min, sec, ms)
                            })
                            .map(|(hr, min, sec, ms)| 
                                (
                                    hr.parse::<u64>().unwrap(), 
                                    min.parse::<u64>().unwrap(), 
                                    sec.parse::<u64>().unwrap(), 
                                    ms.parse::<u64>().unwrap())
                                )
                            .map(|(hr, min, sec, ms)| {
                                (hr * 60 * 60 * 1000) + (min * 60 * 1000) + (sec * 1000) + ms
                            });
                        if let Some(time) = cap {
                            let state = FFmpegState::Downloading(time);
                            if let Err(_err) = ffmpeg_send.send(state.clone()) {
                                let error = FFmpegError {
                                    message: "FFmpeg Channel unexpectedly closed".to_string(),
                                    state: state.clone(),
                                };
                                //NOTE: This is not a problem because errors only occur when another
                                //holder of the mutex panics
                                let mut errors = errors.lock().unwrap();
                                errors.push(error);
                                return;
                            }
                        }
                    });
                ffmpeg_send.send(FFmpegState::Finished).unwrap();
                Ok(())
            }));
            while self.state != FFmpegState::Finished{
                let recv = ffmpeg_recv.recv();
                match recv{
                    Ok(state) => {
                        self.change_state(state.clone());
                        if let FFmpegState::Downloading(time) = state{
                            if let Some(ref cb) = self.on_progress_callback{
                                cb(time);
                            }
                        }
                    },
                    Err(_) => break
                }
            }
        }
        Ok(())
    }
}
