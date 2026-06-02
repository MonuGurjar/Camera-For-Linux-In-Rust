use anyhow::Result;
use gstreamer::prelude::*;
use gstreamer::{ElementFactory, Pipeline, State, Caps};
use gstreamer_app::{AppSink, AppSrc};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crossbeam_channel::{Sender, Receiver};
use image::{RgbaImage, ImageFormat};
use chrono::Local;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CameraCommand {
    StartRecording,
    StopRecording,
    TakePhoto,
    ChangeResolution(i32, i32),
    ChangeFps(i32),
    SetIso(f32),
    SetExposure(f32),
    Quit,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CameraEvent {
    NewFrame(slint::SharedPixelBuffer<slint::Rgba8Pixel>),
    Error(String),
    RecordingTime(String),
}

#[allow(dead_code)]
pub struct CameraBackend {
    command_tx: Sender<CameraCommand>,
}

struct Recorder {
    pipeline: Pipeline,
    appsrc: AppSrc,
}

impl CameraBackend {
    pub fn new(event_tx: Sender<CameraEvent>, command_rx: Receiver<CameraCommand>) -> Result<Self> {
        gstreamer::init()?;
        
        let recorder_mutex: Arc<Mutex<Option<Recorder>>> = Arc::new(Mutex::new(None));
        let take_photo_flag = Arc::new(AtomicBool::new(false));

        let recorder_clone = recorder_mutex.clone();
        let photo_clone = take_photo_flag.clone();
        let event_tx_thread = event_tx.clone();

        std::thread::spawn(move || {
            let mut current_width = 1280;
            let mut current_height = 720;
            let mut current_fps = 30;
            
            let mut active_pipeline: Option<Pipeline> = None;

            let build_preview = |w, h, fps, photo_flag: Arc<AtomicBool>, rec_mutex: Arc<Mutex<Option<Recorder>>>, evt_tx: Sender<CameraEvent>| -> Option<Pipeline> {
                let src = ElementFactory::make("autovideosrc")
                    .build()
                    .or_else(|_| ElementFactory::make("v4l2src").build())
                    .ok()?;

                let capsfilter = ElementFactory::make("capsfilter").build().ok()?;
                let convert = ElementFactory::make("videoconvert").build().ok()?;
                let appsink_element = ElementFactory::make("appsink").build().ok()?;
                let appsink = appsink_element.downcast::<AppSink>().unwrap();

                let pipeline = Pipeline::with_name("camera-preview");
                pipeline.add_many(&[&src, &capsfilter, &convert, appsink.upcast_ref()]).ok()?;
                gstreamer::Element::link_many(&[&src, &capsfilter, &convert, appsink.upcast_ref()]).ok()?;

                let caps = Caps::builder("video/x-raw")
                    .field("width", w)
                    .field("height", h)
                    .field("framerate", gstreamer::Fraction::new(fps, 1))
                    .build();
                capsfilter.set_property("caps", &caps);

                let appsink_caps = Caps::builder("video/x-raw").field("format", "RGBA").build();
                appsink.set_caps(Some(&appsink_caps));
                appsink.set_max_buffers(1);
                appsink.set_drop(true);

                let mut last_record_time = std::time::Instant::now();
                let mut record_seconds = 0;

                appsink.set_callbacks(
                    gstreamer_app::AppSinkCallbacks::builder()
                        .new_sample(move |appsink| {
                            let sample = match appsink.pull_sample() {
                                Ok(s) => s,
                                Err(_) => return Ok(gstreamer::FlowSuccess::Ok),
                            };
                            
                            let buffer = sample.buffer().unwrap();
                            let caps = sample.caps().unwrap();
                            let structure = caps.structure(0).unwrap();
                            let width = structure.get::<i32>("width").unwrap();
                            let height = structure.get::<i32>("height").unwrap();
                            
                            let map = buffer.map_readable().unwrap();
                            let slice = map.as_slice();

                            if photo_flag.swap(false, Ordering::SeqCst) {
                                let img = RgbaImage::from_raw(width as u32, height as u32, slice.to_vec()).unwrap();
                                let rgb_img = image::DynamicImage::ImageRgba8(img).into_rgb8();
                                let filename = format!("photo_{}.jpg", Local::now().format("%Y%m%d_%H%M%S"));
                                let path = dirs::picture_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(filename);
                                rgb_img.save_with_format(&path, ImageFormat::Jpeg).unwrap();
                                let _ = evt_tx.send(CameraEvent::Error(format!("Saved {}", path.display())));
                            }

                            {
                                let mut rec_guard = rec_mutex.lock().unwrap();
                                if let Some(recorder) = rec_guard.as_mut() {
                                    if last_record_time.elapsed().as_secs() >= 1 {
                                        record_seconds += 1;
                                        last_record_time = std::time::Instant::now();
                                        let time_str = format!("{:02}:{:02}", record_seconds / 60, record_seconds % 60);
                                        let _ = evt_tx.send(CameraEvent::RecordingTime(time_str));
                                    }

                                    let mut new_buffer = gstreamer::Buffer::with_size(slice.len()).unwrap();
                                    let new_buffer_mut = new_buffer.get_mut().unwrap();
                                    let mut out_map = new_buffer_mut.map_writable().unwrap();
                                    out_map.as_mut_slice().copy_from_slice(slice);
                                    drop(out_map);
                                    let _ = recorder.appsrc.push_buffer(new_buffer);
                                } else {
                                    record_seconds = 0;
                                    last_record_time = std::time::Instant::now();
                                }
                            }

                            let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width as u32, height as u32);
                            pixel_buffer.make_mut_bytes().copy_from_slice(slice);
                            let _ = evt_tx.send(CameraEvent::NewFrame(pixel_buffer));

                            Ok(gstreamer::FlowSuccess::Ok)
                        })
                        .build(),
                );

                let _ = pipeline.set_state(State::Playing);
                Some(pipeline)
            };

            active_pipeline = build_preview(current_width, current_height, current_fps, photo_clone.clone(), recorder_clone.clone(), event_tx_thread.clone());

            for cmd in command_rx {
                match cmd {
                    CameraCommand::TakePhoto => {
                        photo_clone.store(true, Ordering::SeqCst);
                    }
                    CameraCommand::StartRecording => {
                        let filename = format!("video_{}.mp4", Local::now().format("%Y%m%d_%H%M%S"));
                        let path = dirs::picture_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(filename);
                        
                        match create_recorder_pipeline(&path, current_width, current_height, current_fps) {
                            Ok(rec) => {
                                let _ = rec.pipeline.set_state(State::Playing);
                                *recorder_clone.lock().unwrap() = Some(rec);
                                let _ = event_tx_thread.send(CameraEvent::Error(format!("Recording to {}", path.display())));
                            }
                            Err(e) => {
                                let _ = event_tx_thread.send(CameraEvent::Error(format!("Rec error: {}", e)));
                            }
                        }
                    }
                    CameraCommand::StopRecording => {
                        let mut rec_guard = recorder_clone.lock().unwrap();
                        if let Some(recorder) = rec_guard.take() {
                            let _ = recorder.appsrc.end_of_stream();
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            let _ = recorder.pipeline.set_state(State::Null);
                            let _ = event_tx_thread.send(CameraEvent::Error("Recording stopped".to_string()));
                        }
                    }
                    CameraCommand::ChangeResolution(w, h) => {
                        if let Some(old_pipeline) = active_pipeline.take() {
                            let _ = old_pipeline.set_state(State::Null);
                            let _ = old_pipeline.state(gstreamer::ClockTime::from_mseconds(500));
                            drop(old_pipeline);
                        }
                        
                        current_width = w;
                        current_height = h;
                        
                        active_pipeline = build_preview(current_width, current_height, current_fps, photo_clone.clone(), recorder_clone.clone(), event_tx_thread.clone());
                    }
                    CameraCommand::ChangeFps(fps) => {
                        if let Some(old_pipeline) = active_pipeline.take() {
                            let _ = old_pipeline.set_state(State::Null);
                            let _ = old_pipeline.state(gstreamer::ClockTime::from_mseconds(500));
                            drop(old_pipeline);
                        }
                        
                        current_fps = fps;
                        active_pipeline = build_preview(current_width, current_height, current_fps, photo_clone.clone(), recorder_clone.clone(), event_tx_thread.clone());
                    }
                    CameraCommand::Quit => {
                        if let Some(old_pipeline) = active_pipeline.take() {
                            let _ = old_pipeline.set_state(State::Null);
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Dummy sender for CameraBackend, since the real one handles everything
        let (tx, _) = crossbeam_channel::unbounded();
        Ok(Self { command_tx: tx })
    }
}

fn create_recorder_pipeline(path: &std::path::Path, width: i32, height: i32, fps: i32) -> Result<Recorder> {
    let pipeline = Pipeline::with_name("camera-recorder");
    
    let appsrc_el = ElementFactory::make("appsrc").build()?;
    let appsrc = appsrc_el.downcast::<AppSrc>().unwrap();
    
    let caps = Caps::builder("video/x-raw")
        .field("format", "RGBA")
        .field("width", width)
        .field("height", height)
        .field("framerate", gstreamer::Fraction::new(fps, 1))
        .build();
    
    appsrc.set_caps(Some(&caps));
    appsrc.set_property("is-live", true);
    appsrc.set_property("do-timestamp", true);
    appsrc.set_format(gstreamer::Format::Time);

    let convert = ElementFactory::make("videoconvert").build()?;
    let x264enc = ElementFactory::make("x264enc")
        .build()
        .or_else(|_| ElementFactory::make("jpegenc").build())?; // Fallback
    
    let mux = ElementFactory::make("mp4mux")
        .build()
        .or_else(|_| ElementFactory::make("matroskamux").build())?;
        
    let filesink = ElementFactory::make("filesink")
        .property("location", path.to_str().unwrap())
        .build()?;

    pipeline.add_many(&[appsrc.upcast_ref(), &convert, &x264enc, &mux, &filesink])?;
    gstreamer::Element::link_many(&[appsrc.upcast_ref(), &convert, &x264enc, &mux, &filesink])?;

    Ok(Recorder { pipeline, appsrc })
}

impl Drop for CameraBackend {
    fn drop(&mut self) {
        let _ = self.command_tx.send(CameraCommand::Quit);
    }
}

pub fn probe_capabilities() -> (Vec<String>, Vec<String>) {
    let mut resolutions = Vec::new();
    let mut fps = Vec::new();

    resolutions.push("720p".to_string());
    fps.push("30 FPS".to_string());

    if gstreamer::init().is_err() {
        return (resolutions, fps);
    }

    let monitor = gstreamer::DeviceMonitor::new();
    monitor.add_filter(Some("Video/Source"), None);
    
    let devices = monitor.devices();
    for device in devices {
        if let Some(caps) = device.caps() {
            let test_res = [
                (640, 480, "480p"),
                (1280, 720, "720p"),
                (1920, 1080, "1080p"),
                (2560, 1440, "2K"),
                (3840, 2160, "4K"),
            ];

            let mut supported_res = Vec::new();
            for (w, h, name) in &test_res {
                let test_caps = Caps::builder("video/x-raw")
                    .field("width", *w)
                    .field("height", *h)
                    .build();
                if caps.can_intersect(&test_caps) {
                    supported_res.push(name.to_string());
                }
            }
            
            if !supported_res.is_empty() {
                resolutions = supported_res;
            }

            let test_fps = [
                (30, "30 FPS"),
                (60, "60 FPS"),
            ];
            let mut supported_fps = Vec::new();
            for (f, name) in &test_fps {
                let test_caps = Caps::builder("video/x-raw")
                    .field("framerate", gstreamer::Fraction::new(*f, 1))
                    .build();
                if caps.can_intersect(&test_caps) {
                    supported_fps.push(name.to_string());
                }
            }
            
            if !supported_fps.is_empty() {
                fps = supported_fps;
            }
            
            break;
        }
    }

    (resolutions, fps)
}
