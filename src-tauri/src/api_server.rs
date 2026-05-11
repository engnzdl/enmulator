use crate::adb_bridge;
use crate::avd_manager;
use crate::config::Config;
use crate::device::{Device, DeviceStore};
use crate::emulator::EmulatorStore;
use crate::fingerprint;
use crate::sdk;

use actix_web::{web, App, HttpResponse, HttpServer};
use bytes::Bytes;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::sync::mpsc;

// ── Shared application state ─────────────────────────────────────────────────

pub struct AppState {
    pub device_store: Arc<DeviceStore>,
    pub emulator_store: Arc<EmulatorStore>,
    pub config: Arc<Mutex<Config>>,
}

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub profile: Option<String>,
    pub api_level: u8,
    pub abi: Option<String>,
    pub tag: Option<String>,
}

#[derive(Deserialize)]
pub struct AdbRequest {
    pub cmd: String,
}

#[derive(Serialize)]
struct ApiError {
    success: bool,
    error: String,
}

fn err(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(ApiError { success: false, error: msg.to_string() })
}

fn not_found(msg: &str) -> HttpResponse {
    HttpResponse::NotFound().json(ApiError { success: false, error: msg.to_string() })
}

fn ok_json<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

// ── Route handlers ───────────────────────────────────────────────────────────

// POST /api/devices
async fn create_device(
    state: web::Data<AppState>,
    body: web::Json<CreateDeviceRequest>,
) -> HttpResponse {
    let sdk_path = match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => PathBuf::from(p),
        None => return err("SDK not configured"),
    };
    let profile = body.profile.clone().unwrap_or_else(|| "pixel_5".to_string());
    let device_id = body.name.to_lowercase().replace(' ', "_");
    let abi = body.abi.clone().unwrap_or_else(|| "x86_64".to_string());
    let tag = body.tag.clone().unwrap_or_else(|| "google_apis".to_string());

    match avd_manager::create_avd(
        &sdk_path,
        &device_id,
        &body.name,
        body.api_level,
        &abi,
        &tag,
        &profile,
        None,
        &state.device_store.devices_dir,
    ) {
        Ok(dev) => {
            state.device_store.insert(dev.clone());
            ok_json(dev)
        }
        Err(e) => err(&e),
    }
}

// GET /api/devices
async fn list_devices(state: web::Data<AppState>) -> HttpResponse {
    ok_json(state.device_store.list())
}

// POST /api/devices/{id}/start
async fn start_device(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => PathBuf::from(p),
        None => return err("SDK not configured"),
    };
    let dev = match state.device_store.get(&id) {
        Some(d) => d,
        None => return not_found("Device not found"),
    };
    match state.emulator_store.start(&sdk_path, &dev.avd_name, true) {
        Ok(port) => {
            state.device_store.update_port(&id, port);
            state.device_store.update_status(&id, "running");
            ok_json(serde_json::json!({ "port": port }))
        }
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/stop
async fn stop_device(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => PathBuf::from(p),
        None => return err("SDK not configured"),
    };
    let dev = match state.device_store.get(&id) {
        Some(d) => d,
        None => return not_found("Device not found"),
    };
    state.emulator_store.stop(&sdk_path, &dev.avd_name, dev.port);
    state.device_store.update_status(&id, "stopped");
    ok_json(serde_json::json!({ "status": "stopped" }))
}

// DELETE /api/devices/{id}
async fn delete_device(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    if state.device_store.get(&id).is_none() {
        return not_found("Device not found");
    }
    state.device_store.remove(&id);
    ok_json(serde_json::json!({ "deleted": id }))
}

// POST /api/devices/{id}/clone
#[derive(Deserialize)]
struct CloneRequest {
    target_name: String,
}

async fn clone_device(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CloneRequest>,
) -> HttpResponse {
    let source_id = path.into_inner();
    let target_name = body.target_name.clone();

    let source = match state.device_store.get(&source_id) {
        Some(d) => d,
        None => return not_found("Source device not found"),
    };

    let target_id = target_name.to_lowercase().replace(' ', "_");
    let src_dir = state.device_store.devices_dir.join(&source_id);
    let dst_dir = state.device_store.devices_dir.join(&target_id);

    fn copy_dir(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let dest = dst.join(entry.file_name());
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "snapshots" {
                    copy_dir(&entry.path(), &dest)?;
                }
            } else {
                std::fs::copy(entry.path(), &dest).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    if let Err(e) = copy_dir(&src_dir, &dst_dir) {
        return err(&e);
    }

    let cloned = Device {
        id: target_id.clone(),
        display_name: target_name,
        avd_name: format!("enmulator_{}", target_id),
        profile: source.profile.clone(),
        fingerprint_profile: source.fingerprint_profile.clone(),
        api_level: source.api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: source.root_enabled,
        adb_enabled: source.adb_enabled,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.device_store.insert(cloned.clone());
    ok_json(cloned)
}

// POST /api/devices/{id}/adb
async fn adb_shell(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<AdbRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => PathBuf::from(p),
        None => return err("SDK not configured"),
    };
    let dev = match state.device_store.get(&id) {
        Some(d) => d,
        None => return not_found("Device not found"),
    };
    let serial = format!("emulator-{}", dev.port);
    match adb_bridge::shell(&sdk_path, &serial, &body.cmd) {
        Ok(output) => ok_json(serde_json::json!({ "output": output })),
        Err(e) => err(&e),
    }
}

// GET /api/profiles
async fn list_profiles() -> HttpResponse {
    let profiles = fingerprint::list_profiles(&PathBuf::from("profiles"));
    ok_json(profiles)
}

// ── SSE logcat stream ────────────────────────────────────────────────────────

/// A simple Stream wrapper around an mpsc receiver.
struct MpscStream {
    rx: mpsc::Receiver<Bytes>,
}

impl Stream for MpscStream {
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx).map(|opt| opt.map(Ok))
    }
}

// GET /api/devices/{id}/logcat  (SSE)
async fn logcat_stream(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();

    let sdk_path = match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => PathBuf::from(p),
        None => return err("SDK not configured"),
    };
    let dev = match state.device_store.get(&id) {
        Some(d) => d,
        None => return not_found("Device not found"),
    };

    let serial = format!("emulator-{}", dev.port);
    let adb = sdk::get_adb_path(&sdk_path);

    let (tx, rx) = mpsc::channel::<Bytes>(32);

    // Spawn a blocking thread to run `adb logcat` and forward lines as SSE events.
    std::thread::spawn(move || {
        use std::io::BufRead;
        use std::process::{Command, Stdio};

        let mut child = match Command::new(&adb)
            .args(["-s", &serial, "logcat"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.blocking_send(Bytes::from(
                    format!("event: error\ndata: {}\n\n", e),
                ));
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    let sse = format!("data: {}\n\n", text);
                    if tx.blocking_send(Bytes::from(sse)).is_err() {
                        break; // client disconnected
                    }
                }
                Err(_) => break,
            }
        }
        let _ = child.kill();
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(MpscStream { rx })
}

// ── Route configuration ──────────────────────────────────────────────────────

fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/devices", web::post().to(create_device))
            .route("/devices", web::get().to(list_devices))
            .route("/devices/{id}/start", web::post().to(start_device))
            .route("/devices/{id}/stop", web::post().to(stop_device))
            .route("/devices/{id}", web::delete().to(delete_device))
            .route("/devices/{id}/clone", web::post().to(clone_device))
            .route("/devices/{id}/adb", web::post().to(adb_shell))
            .route("/devices/{id}/logcat", web::get().to(logcat_stream))
            .route("/profiles", web::get().to(list_profiles)),
    );
}

// ── Server lifecycle management ──────────────────────────────────────────────

static SERVER_HANDLE: Mutex<Option<actix_web::dev::ServerHandle>> = Mutex::new(None);

/// Start the REST API server on the given port.  Spawns the server onto a
/// dedicated OS thread so the Tauri main thread stays unblocked.
pub fn start_api_server(state: Arc<AppState>, port: u16) -> Result<(), String> {
    let mut guard = SERVER_HANDLE.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("API server is already running".to_string());
    }

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(state.clone()))
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", port))
    .map_err(|e| format!("Failed to bind port {}: {}", port, e))?
    .run();

    let handle = server.handle();
    *guard = Some(handle);

    // Run the server on a dedicated thread.
    std::thread::spawn(move || {
        let rt = actix_rt::System::new();
        rt.block_on(server).ok();
    });

    Ok(())
}

/// Stop a running REST API server.
pub fn stop_api_server() -> Result<(), String> {
    let mut guard = SERVER_HANDLE.lock().map_err(|e| e.to_string())?;
    match guard.take() {
        Some(handle) => {
            let rt = actix_rt::System::new();
            rt.block_on(handle.stop(true));
            Ok(())
        }
        None => Err("API server is not running".to_string()),
    }
}
