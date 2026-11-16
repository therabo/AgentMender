//! Loopback-only transport and browser authorization for the local UI.
mod browser;
mod inventory;
mod removal;
mod transcript;

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use agentmender_domain::ProviderKind;
use agentmender_provider_api::process;
use agentmender_provider_claude::ClaudeAdapter;
use agentmender_provider_codex::CodexAdapter;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use self::{
    inventory::{DetailView, Filter, Inventories, PageView, ProviderView},
    removal::{NativeProfile, OperationStatus, OperationView, Operations, PreviewView},
    transcript::{TranscriptCursors, TranscriptQuery, TranscriptView},
};
use crate::{Args, CliError, ErrorCode};

const INDEX: &str = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
const SCRIPT: &str = include_str!(concat!(env!("OUT_DIR"), "/app.js"));
const STYLES: &str = include_str!(concat!(env!("OUT_DIR"), "/app.css"));

struct App {
    host: String,
    origin: String,
    launch: Mutex<Option<String>>,
    authorization: String,
    codex_root: PathBuf,
    claude_root: PathBuf,
    codex: CodexAdapter,
    claude: ClaudeAdapter,
    limit: usize,
    protected: Vec<String>,
    native: Option<NativeProfile>,
    inventory: Inventories,
    operations: Mutex<Operations>,
    cursors: Mutex<TranscriptCursors>,
    reads: Arc<Semaphore>,
}

#[derive(Debug)]
struct ApiError(StatusCode, &'static str);

impl ApiError {
    fn bad(message: &'static str) -> Self {
        Self(StatusCode::BAD_REQUEST, message)
    }
    fn conflict(message: &'static str) -> Self {
        Self(StatusCode::CONFLICT, message)
    }
    fn missing(message: &'static str) -> Self {
        Self(StatusCode::NOT_FOUND, message)
    }
    fn internal() -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Operazione locale non disponibile.",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

fn token() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| ApiError::internal())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn header_matches(headers: &HeaderMap, name: HeaderName, expected: &str) -> bool {
    let mut values = headers.get_all(name).iter();
    values.next().is_some_and(|value| value == expected) && values.next().is_none()
}

async fn boundary(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let path = request.uri().path();
    let authorized = || header_matches(headers, header::AUTHORIZATION, &app.authorization);
    let origin = || header_matches(headers, header::ORIGIN, &app.origin);
    let mut response = if !header_matches(headers, header::HOST, &app.host)
        || (!matches!(*request.method(), Method::GET | Method::HEAD) && !origin())
        || (headers.contains_key(header::ORIGIN) && !origin())
    {
        ApiError(StatusCode::FORBIDDEN, "Origine locale non valida.").into_response()
    } else if path.starts_with("/api/") && path != "/api/authorize" && !authorized() {
        ApiError(
            StatusCode::UNAUTHORIZED,
            "Apri il link di avvio di AgentMender.",
        )
        .into_response()
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    for (name, value) in [
        ("cache-control", "no-store"),
        ("referrer-policy", "no-referrer"),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("cross-origin-resource-policy", "same-origin"),
        (
            "content-security-policy",
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
        ),
    ] {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchRequest {
    ticket: String,
}

#[derive(Serialize)]
struct BrowserSession {
    authorization: String,
}

async fn authorize(
    State(app): State<Arc<App>>,
    Json(request): Json<LaunchRequest>,
) -> Result<Json<BrowserSession>, ApiError> {
    let mut ticket = app.launch.lock().expect("launch lock");
    if ticket.as_ref() != Some(&request.ticket) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Link di avvio non valido o già utilizzato.",
        ));
    }
    *ticket = None;
    Ok(Json(BrowserSession {
        authorization: app.authorization.clone(),
    }))
}

async fn read<T: Send + 'static>(
    app: Arc<App>,
    action: impl FnOnce(&App) -> Result<T, ApiError> + Send + 'static,
) -> Result<Json<T>, ApiError> {
    let slot = app.reads.clone().try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "AgentMender è occupato. Riprova tra poco.",
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _slot = slot;
        action(&app).map(Json)
    })
    .await
    .map_err(|_| ApiError::internal())?
}

fn provider(value: &str) -> Result<ProviderKind, ApiError> {
    match value {
        "codex" => Ok(ProviderKind::Codex),
        "claude" => Ok(ProviderKind::Claude),
        _ => Err(ApiError::missing("Provider non riconosciuto.")),
    }
}

async fn providers(State(app): State<Arc<App>>) -> Result<Json<[ProviderView; 2]>, ApiError> {
    read(app, |app| {
        Ok([
            app.inventory.info(app, ProviderKind::Codex),
            app.inventory.info(app, ProviderKind::Claude),
        ])
    })
    .await
}

async fn list(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Query(filter): Query<Filter>,
) -> Result<Json<PageView>, ApiError> {
    let provider = provider(&id)?;
    let filter = filter.try_into()?;
    read(app, move |app| {
        Ok(app.inventory.list(app, provider, &filter))
    })
    .await
}

async fn detail(
    State(app): State<Arc<App>>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<DetailView>, ApiError> {
    let provider = provider(&id)?;
    read(app, move |app| app.inventory.detail(app, provider, &key)).await
}

async fn refresh(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Result<Json<ProviderView>, ApiError> {
    let provider = provider(&id)?;
    read(app, move |app| {
        app.inventory.invalidate(provider);
        Ok(app.inventory.info(app, provider))
    })
    .await
}

async fn conversation(
    State(app): State<Arc<App>>,
    Path((id, key)): Path<(String, String)>,
    Query(query): Query<TranscriptQuery>,
) -> Result<Json<TranscriptView>, ApiError> {
    let provider = provider(&id)?;
    read(app, move |app| transcript::page(app, provider, &key, query)).await
}

async fn preview(
    State(app): State<Arc<App>>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<PreviewView>, ApiError> {
    if provider(&id)? != ProviderKind::Codex {
        return Err(ApiError::conflict(inventory::NO_CLAUDE_REMOVAL));
    }
    read(app, move |app| {
        let session = app.inventory.selected(app, ProviderKind::Codex, &key)?;
        if let Some(reason) = removal::refusal(&session, app.native.is_some(), &app.protected) {
            return Err(ApiError::conflict(reason));
        }
        let native = app
            .native
            .as_ref()
            .ok_or_else(|| ApiError::conflict(removal::UNSUPPORTED))?;
        let evidence = native.prepare(&app.codex, &session)?;
        app.operations
            .lock()
            .expect("operation lock")
            .issue(evidence)
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    confirmation: String,
}

async fn confirm(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    Json(request): Json<Confirmation>,
) -> Result<StatusCode, ApiError> {
    if app.native.is_none() {
        return Err(ApiError::conflict(removal::UNSUPPORTED));
    }
    let review = app
        .operations
        .lock()
        .expect("operation lock")
        .start(&key, &request.confirmation)?;
    tokio::spawn(async move {
        let worker = app.clone();
        let status = tokio::task::spawn_blocking(move || {
            let native = worker.native.as_ref().expect("checked");
            if worker
                .protected
                .contains(&review.session.metadata.native_session_id)
            {
                OperationStatus::Changed
            } else {
                let status = native.execute(&worker.codex, &review);
                worker.inventory.invalidate(ProviderKind::Codex);
                status
            }
        })
        .await
        .unwrap_or(OperationStatus::OutcomeUnknown);
        app.operations
            .lock()
            .expect("operation lock")
            .finish(&key, status);
    });
    Ok(StatusCode::ACCEPTED)
}

async fn operation(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<OperationView>, ApiError> {
    app.operations
        .lock()
        .expect("operation lock")
        .get(&key)
        .map(Json)
        .ok_or_else(|| ApiError::missing("Operazione non trovata nella sessione locale."))
}

fn asset(content_type: &'static str, bytes: &'static str) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, content_type)], bytes)
}

fn router(app: Arc<App>) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { asset("text/html; charset=utf-8", INDEX) }),
        )
        .route(
            "/app.js",
            get(|| async { asset("text/javascript; charset=utf-8", SCRIPT) }),
        )
        .route(
            "/app.css",
            get(|| async { asset("text/css; charset=utf-8", STYLES) }),
        )
        .route("/favicon.ico", get(|| async { StatusCode::NO_CONTENT }))
        .route("/api/authorize", post(authorize))
        .route("/api/providers", get(providers))
        .route("/api/{provider}/sessions", get(list))
        .route("/api/{provider}/sessions/{key}", get(detail))
        .route(
            "/api/{provider}/sessions/{key}/conversation",
            get(conversation),
        )
        .route(
            "/api/{provider}/sessions/{key}/removal-preview",
            post(preview),
        )
        .route("/api/{provider}/refresh", post(refresh))
        .route("/api/operations/{key}", get(operation))
        .route("/api/operations/{key}/confirm", post(confirm))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(app.clone(), boundary))
        .with_state(app)
}

pub(super) fn run(args: &Args, no_open: bool) -> Result<bool, CliError> {
    let error = |message| CliError::new(ErrorCode::InvalidConfiguration, message);
    if args.fixture.is_some()
        || args.provider.is_some()
        || args.project.is_some()
        || args.active_only
        || args.search_index_root.is_some()
        || args.codex_desktop_user_data.is_some()
    {
        return Err(error(
            "UI uses both provider roots and its own metadata filters. Supply --codex-root and --claude-root for alternate archives.",
        ));
    }
    if INDEX.is_empty() || SCRIPT.is_empty() || STYLES.is_empty() {
        return Err(error(
            "UI assets are missing. Run npm ci and npm run build in ui/, then rebuild AgentMender.",
        ));
    }
    let mut configured = args.clone();
    let codex_root = crate::root(args.codex_root.clone(), "CODEX_HOME", ".codex")?;
    configured.codex_root = Some(codex_root.clone());
    let claude_root = crate::root(args.claude_root.clone(), "CLAUDE_CONFIG_DIR", ".claude")?;
    configured.codex_executable = configured
        .codex_executable
        .or_else(|| process::find_executable(ProviderKind::Codex))
        .or_else(process::find_codex_desktop_executable);
    let native = NativeProfile::detect(configured.codex_executable.as_deref(), &codex_root);
    let codex = crate::codex_adapter(&configured, None)?;
    let claude = crate::claude_adapter(&configured, None)?;
    let launch = token().map_err(|_| error("Unable to create a local authorization ticket."))?;
    let bearer = token().map_err(|_| error("Unable to create a local browser session."))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| error("Unable to start the local HTTP runtime."))?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| error("Unable to bind the local HTTP listener."))?;
        let addr: SocketAddr = listener
            .local_addr()
            .map_err(|_| error("Unable to read the local address."))?;
        let origin = format!("http://{addr}");
        let url = format!("{origin}/#ticket={launch}");
        let app = Arc::new(App {
            host: addr.to_string(),
            origin,
            launch: Mutex::new(Some(launch.clone())),
            authorization: format!("Bearer {bearer}"),
            codex_root,
            claude_root,
            codex,
            claude,
            native,
            limit: args.max_entries,
            protected: args.protect_codex_session.clone(),
            inventory: Inventories::default(),
            operations: Mutex::default(),
            cursors: Mutex::default(),
            reads: Arc::new(Semaphore::new(8)),
        });
        println!("{url}");
        if !no_open && browser::open(addr, &launch).is_err() {
            eprintln!("Open the local URL printed above in your browser.");
        }
        axum::serve(listener, router(app))
            .await
            .map_err(|_| error("The local HTTP server stopped unexpectedly."))?;
        Ok(false)
    })
}
