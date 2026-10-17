use crate::adb_bridge;
use crate::avd_manager;
use crate::config::Config;
use crate::device::{Device, DeviceStore};
use crate::emulator::EmulatorStore;
use crate::extras::{self, RecordingStore};
use crate::fingerprint;
use crate::paths;
use crate::sdk;
use crate::set_proxy::{ProxyConfig, ProxyStore};

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
    pub proxy_store: Arc<ProxyStore>,
    pub rec_store: Arc<RecordingStore>,
}

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
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
        None,
        None,
        None,
        None,
        false,
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

    let target_id = crate::avd_manager::sanitize_id(&target_name);
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

    let new_avd_name = format!("enmulator_{}", target_id);
    if let Err(e) = crate::avd_manager::register_avd_at_path(&new_avd_name, &dst_dir) {
        return err(&e);
    }

    let cloned = Device {
        id: target_id.clone(),
        display_name: target_name,
        avd_name: new_avd_name,
        profile: source.profile.clone(),
        fingerprint_profile: source.fingerprint_profile.clone(),
        api_level: source.api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: false,
        adb_enabled: source.adb_enabled,
        writable_system: false,
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
    let profiles = fingerprint::list_profiles(&paths::profiles_dir());
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
        use std::process::Stdio;

        let mut child = match sdk::new_command(&adb)
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

// ── Control endpoints (identity, install, gps, proxy, hide-root, screenshot, record) ──

fn sdk_path_of(state: &AppState) -> Result<PathBuf, String> {
    match state.config.lock().unwrap().sdk_path.clone() {
        Some(p) => Ok(PathBuf::from(p)),
        None => Err("SDK not configured".into()),
    }
}

fn resolve_serial(state: &AppState, id: &str) -> Result<String, String> {
    let dev = state.device_store.get(id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    Ok(format!("emulator-{}", dev.port))
}

#[derive(Deserialize)]
struct IdentityRequest {
    profile_name: String,
}

#[derive(Deserialize)]
struct InstallRequest {
    apk_path: String,
}

#[derive(Deserialize)]
struct GpsRequest {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct ProxyRequest {
    host: String,
    port: u16,
    enabled: bool,
}

#[derive(Deserialize)]
struct HideRootRequest {
    #[serde(default)]
    packages: Vec<String>,
}

// POST /api/devices/{id}/identity — apply a fingerprint profile
async fn apply_identity(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<IdentityRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    let profiles = fingerprint::list_profiles(&paths::profiles_dir());
    let profile = match profiles.into_iter().find(|p| p.name == body.profile_name) {
        Some(p) => p,
        None => return not_found("Profile not found"),
    };
    match fingerprint::apply_to_device(&sdk_path, &serial, &profile) {
        Ok(summary) => ok_json(serde_json::json!({ "summary": summary })),
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/install — install an APK
async fn install_apk(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<InstallRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    match adb_bridge::install_apk(&sdk_path, &serial, &body.apk_path) {
        Ok(out) => ok_json(serde_json::json!({ "output": out })),
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/gps — mock GPS location
async fn gps_set(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<GpsRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    match extras::set_gps(&sdk_path, &serial, body.lat, body.lon) {
        Ok(()) => ok_json(serde_json::json!({ "status": "ok" })),
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/proxy — set per-device HTTP proxy
async fn set_proxy(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<ProxyRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    if body.enabled {
        let cmd = format!("settings put global http_proxy {}:{}", body.host, body.port);
        if let Err(e) = adb_bridge::shell(&sdk_path, &serial, &cmd) {
            return err(&e);
        }
        state.proxy_store.set(&id, ProxyConfig { host: body.host.clone(), port: body.port, enabled: true });
    } else {
        if let Err(e) = adb_bridge::shell(&sdk_path, &serial, "settings put global http_proxy :0") {
            return err(&e);
        }
        state.proxy_store.remove(&id);
    }
    ok_json(serde_json::json!({ "status": "ok" }))
}

// POST /api/devices/{id}/hide-root — Magisk DenyList + Zygisk + NoHello
async fn hide_root(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<HideRootRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    match adb_bridge::hide_root_core(&sdk_path, &serial, &body.packages) {
        Ok(summary) => ok_json(serde_json::json!({ "summary": summary })),
        Err(e) => err(&e),
    }
}

// GET /api/devices/{id}/screenshot — capture a PNG screenshot
async fn screenshot(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    let tmp = std::env::temp_dir().join(format!("enmulator_screenshot_{}.png", id));
    match adb_bridge::screenshot(&sdk_path, &serial, &tmp.to_string_lossy()) {
        Ok(()) => match std::fs::read(&tmp) {
            Ok(bytes) => {
                let _ = std::fs::remove_file(&tmp);
                HttpResponse::Ok().content_type("image/png").body(bytes)
            }
            Err(e) => err(&e.to_string()),
        },
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/record/start
async fn record_start(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    match extras::start_recording(&sdk_path, &serial, &id, &state.rec_store) {
        Ok(()) => ok_json(serde_json::json!({ "status": "recording" })),
        Err(e) => err(&e),
    }
}

// POST /api/devices/{id}/record/stop
async fn record_stop(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let sdk_path = match sdk_path_of(&state) {
        Ok(p) => p,
        Err(e) => return err(&e),
    };
    let serial = match resolve_serial(&state, &id) {
        Ok(s) => s,
        Err(e) => return err(&e),
    };
    let out_dir = std::env::temp_dir();
    match extras::stop_recording(&sdk_path, &serial, &id, &state.rec_store, &out_dir) {
        Ok(local) => ok_json(serde_json::json!({ "path": local.to_string_lossy() })),
        Err(e) => err(&e),
    }
}

// ── Route configuration ──────────────────────────────────────────────────────

fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .guard(actix_web::guard::fn_guard(token_guard))
            .route("/devices", web::post().to(create_device))
            .route("/devices", web::get().to(list_devices))
            .route("/devices/{id}/start", web::post().to(start_device))
            .route("/devices/{id}/stop", web::post().to(stop_device))
            .route("/devices/{id}", web::delete().to(delete_device))
            .route("/devices/{id}/clone", web::post().to(clone_device))
            .route("/devices/{id}/adb", web::post().to(adb_shell))
            .route("/devices/{id}/logcat", web::get().to(logcat_stream))
            .route("/devices/{id}/identity", web::post().to(apply_identity))
            .route("/devices/{id}/install", web::post().to(install_apk))
            .route("/devices/{id}/gps", web::post().to(gps_set))
            .route("/devices/{id}/proxy", web::post().to(set_proxy))
            .route("/devices/{id}/hide-root", web::post().to(hide_root))
            .route("/devices/{id}/screenshot", web::get().to(screenshot))
            .route("/devices/{id}/record/start", web::post().to(record_start))
            .route("/devices/{id}/record/stop", web::post().to(record_stop))
            .route("/profiles", web::get().to(list_profiles)),
    );
}

// ── Server lifecycle management ──────────────────────────────────────────────

/// Auth token for the REST API. Populated when the server starts, cleared on stop.
/// Every /api route is gated on the `x-api-token` header matching this value,
/// which blocks cross-site (CSRF) requests from arbitrary web pages.
static API_TOKEN: Mutex<Option<String>> = Mutex::new(None);

fn token_guard(ctx: &actix_web::guard::GuardContext) -> bool {
    let Ok(guard) = API_TOKEN.lock() else { return false; };
    let Some(token) = guard.as_deref() else { return true; }; // server not started — allow
    ctx.head()
        .headers()
        .get("x-api-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == token)
        .unwrap_or(false)
}

static SERVER_HANDLE: Mutex<Option<actix_web::dev::ServerHandle>> = Mutex::new(None);

/// Start the REST API server on the given port.  Spawns the server onto a
/// dedicated OS thread so the Tauri main thread stays unblocked.
pub fn start_api_server(state: Arc<AppState>, port: u16) -> Result<(), String> {
    let mut guard = SERVER_HANDLE.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("API server is already running".to_string());
    }

    // Publish the auth token so the route guard can enforce it.
    let token = state.config.lock().map_err(|e| e.to_string())?.api_token.clone();
    *API_TOKEN.lock().map_err(|e| e.to_string())? = Some(token);

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
            *API_TOKEN.lock().map_err(|e| e.to_string())? = None;
            Ok(())
        }
        None => Err("API server is not running".to_string()),
    }
}
