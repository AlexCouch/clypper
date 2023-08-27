use std::{error::Error, fmt::Write, cmp::{min, max}};

use download::{downloader::Downloader, ffmpeg::{FFmpegError, FFmpeg}};
use extract::extractor::{ClypperError, ClipTime};
use indicatif::{ProgressBar, ProgressStyle, ProgressState};

use crate::{extract::extractor::{Extractor, ClipResource}, download::ffmpeg::FFmpegState};

mod extract;
mod download;

#[test]
fn test_extractor() -> Result<(), ClypperError> {
    let url = "https://www.youtube.com/clip/UgkxWw82JHM2Y6ZFPbT9lIVAgOzgbWEl-ocz";
    let extractor = Extractor::new().unwrap();
    let clip = match extractor.extract(url){
        Ok(clip) => clip,
        Err(err) => {
            println!("{:?}", err);
            return Err(err);
        }
    };
    let ClipResource(_vid_url, _aud_url) = clip.resource;
    let ClipTime(_start, _end) = clip.time;

    Ok(())
}

#[test]
fn test_downloader() -> Result<(), ClypperError>{
    let url = "https://www.youtube.com/clip/UgkxWw82JHM2Y6ZFPbT9lIVAgOzgbWEl-ocz";
    let extractor = Extractor::new().unwrap();
    let clip = match extractor.extract(url){
        Ok(clip) => clip,
        Err(err) => {
            println!("{:?}", err);
            return Err(err);
        }
    };
    
    let mut downloader = Downloader::new(clip, String::from("test.mp4"));
    downloader.download()?;
    Ok(())
}

#[test]
fn test_ffmpeg() -> Result<(), FFmpegError>{
    let url = "https://www.youtube.com/clip/UgkxWw82JHM2Y6ZFPbT9lIVAgOzgbWEl-ocz";
    let extractor = Extractor::new().unwrap();
    let clip = match extractor.extract(url){
        Ok(clip) => clip,
        Err(err) => {
            println!("{:?}", err);
            return Err(FFmpegError{ message: "Failed to extract clip info".to_string(), state: FFmpegState::NotStarted });
        }
    };
    
    println!("{:?}", clip.time.clone());
    let total = clip.time.1 - clip.time.0;
    let pb = ProgressBar::new(total)
        .with_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:100.cyan/blue} {msg} {pos}/{len} ({eta})")
        .unwrap());
    let pb_cb = pb.clone();
    let pb_cb2 = pb.clone();
    let mut ffmpeg = FFmpeg::new();
    ffmpeg.time(clip.time.0, clip.time.1)?
        .input(&clip.resource.0)?
        .input(&clip.resource.1)?
        .output("test.mp4")?
        .state_change_callback(move |state|{
            match state{
                FFmpegState::Starting => pb_cb2.set_message("FFmpeg starting..."),
                FFmpegState::Downloading(_) => pb_cb2.set_message("Downloading..."),
                FFmpegState::Finished => pb_cb2.set_message("Done!"),
                _ => {}
            }
        })
        .progress_callback(move |time|{
            pb_cb.set_message("Downloading...");
            pb_cb.set_position(time);
        })
        .spawn()?;

    pb.finish();
    
    
    Ok(())
}
