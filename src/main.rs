slint::include_modules!();

mod camera;
mod power;

use camera::{CameraBackend, CameraCommand, CameraEvent};
use crossbeam_channel::unbounded;
use std::thread;

fn main() -> Result<(), anyhow::Error> {
    let ui = AppWindow::new()?;

    // Probe hardware capabilities
    let (res, fps) = camera::probe_capabilities();
    
    use slint::{ModelRc, VecModel};
    use std::rc::Rc;
    
    let res_model = Rc::new(VecModel::from(res.into_iter().map(slint::SharedString::from).collect::<Vec<_>>()));
    ui.set_available_resolutions(ModelRc::from(res_model.clone()));
    
    let fps_model = Rc::new(VecModel::from(fps.into_iter().map(slint::SharedString::from).collect::<Vec<_>>()));
    ui.set_available_fps(ModelRc::from(fps_model.clone()));

    // Start Power Monitor
    thread::spawn(move || {
        futures_lite::future::block_on(power::monitor_power_events()).unwrap();
    });

    let (cmd_tx, cmd_rx) = unbounded();
    let (event_tx, event_rx) = unbounded();

    // Initialize Camera Backend on a separate thread (or just initialize here and let its callbacks run on GStreamer threads)
    let _camera_backend = CameraBackend::new(event_tx.clone(), cmd_rx)?;

    // Handle events from Camera Backend to UI
    slint::invoke_from_event_loop(move || {
        // We'd ideally use a timer or a dedicated channel receiver mechanism integrated with Slint event loop
        // For simplicity, we can spawn a thread that receives events and invokes them on UI.
    }).unwrap();

    let ui_handle_clone = ui.as_weak();
    std::thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            match event {
                CameraEvent::NewFrame(buffer) => {
                    let _ = ui_handle_clone.upgrade_in_event_loop(move |ui| {
                        let image = slint::Image::from_rgba8(buffer);
                        ui.set_video_frame(image);
                    });
                }
                CameraEvent::Error(err) => {
                    let _ = ui_handle_clone.upgrade_in_event_loop(move |ui| {
                        ui.set_toast_message(err.into());
                    });
                }
                CameraEvent::RecordingTime(time_str) => {
                    let _ = ui_handle_clone.upgrade_in_event_loop(move |ui| {
                        ui.set_recording_time(time_str.into());
                    });
                }
            }
        }
    });

    // Handle UI callbacks
    let cmd_tx_photo = cmd_tx.clone();
    ui.on_take_photo(move || {
        cmd_tx_photo.send(CameraCommand::TakePhoto).unwrap();
    });

    let cmd_tx_rec = cmd_tx.clone();
    let ui_rec = ui.as_weak();
    ui.on_toggle_recording(move || {
        if let Some(ui) = ui_rec.upgrade() {
            let is_recording = ui.get_is_recording();
            if is_recording {
                cmd_tx_rec.send(CameraCommand::StopRecording).unwrap();
                ui.set_is_recording(false);
            } else {
                cmd_tx_rec.send(CameraCommand::StartRecording).unwrap();
                ui.set_is_recording(true);
            }
        }
    });

    let ui_borderless = ui.as_weak();
    ui.on_toggle_borderless(move || {
        if let Some(ui) = ui_borderless.upgrade() {
            ui.set_is_borderless(!ui.get_is_borderless());
        }
    });

    let cmd_tx_res = cmd_tx.clone();
    ui.on_change_resolution(move |res| {
        let (w, h) = match res.as_str() {
            "480p" => (640, 480),
            "720p" => (1280, 720),
            "1080p" => (1920, 1080),
            "2K" => (2560, 1440),
            "4K" => (3840, 2160),
            _ => (1280, 720),
        };
        cmd_tx_res.send(CameraCommand::ChangeResolution(w, h)).unwrap();
    });

    let cmd_tx_fps = cmd_tx.clone();
    ui.on_change_fps(move |fps| {
        let val = match fps.as_str() {
            "30 FPS" => 30,
            "60 FPS" => 60,
            _ => 60,
        };
        cmd_tx_fps.send(CameraCommand::ChangeFps(val)).unwrap();
    });

    ui.run()?;
    Ok(())
}
