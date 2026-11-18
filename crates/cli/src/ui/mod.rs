//! Loopback-only transport and browser authorization for the local UI.
mod browser;
mod claude_usage;
mod inventory;
mod removal;
mod storage;
mod transcript;

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use agentmender_domain::ProviderKind;
use agentmender_provider_claude::ClaudeAdapter;
use agentmender_provider_codex::{CodexAdapter, discovery::Discovery};
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
    inventory::{
        DetailView, Filter, Inventories, PageView, ProjectView, ProjectsPage, ProviderView,
    },
    transcript::{TranscriptCursors, TranscriptQuery, TranscriptView},
};

const INDEX: &str = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
const SCRIPT: &str = include_str!(concat!(env!("OUT_DIR"), "/app.js"));
const STYLES: &str = include_str!(concat!(env!("OUT_DIR"), "/app.css"));

struct App {
    host: String,
    origin: String,
    launch: Mutex<Option<String>>,
    authorization: String,
    codex_root: PathBuf,
    codex: CodexAdapter,
    claude: ClaudeAdapter,
    limit: usize,
    protected: Vec<String>,
    inventory: Inventories,
    discovery: Discovery,
    exports: Mutex<Option<storage::CachedInventory>>,
    claude_usage: Mutex<Option<claude_usage::CachedUsage>>,
    #[cfg(target_os = "macos")]
    claude_review: Mutex<Option<removal::ClaudeReview>>,
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

async fn recents(
    State(app): State<Arc<App>>,
    Query(filter): Query<Filter>,
) -> Result<Json<PageView>, ApiError> {
    let query = filter.try_into()?;
    read(app, move |app| Ok(app.inventory.recents(app, &query))).await
}

async fn projects(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Query(filter): Query<Filter>,
) -> Result<Json<ProjectsPage>, ApiError> {
    let provider = provider(&id)?;
    let query = filter.try_into()?;
    read(app, move |app| {
        Ok(app.inventory.projects(app, provider, &query))
    })
    .await
}

async fn project_detail(
    State(app): State<Arc<App>>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<ProjectView>, ApiError> {
    let provider = provider(&id)?;
    read(app, move |app| {
        app.inventory
            .project(app, provider, &key, |app, _, project| {
                ProjectView::new(app, project)
            })
    })
    .await
}

async fn project_sessions(
    State(app): State<Arc<App>>,
    Path((id, key)): Path<(String, String)>,
    Query(filter): Query<Filter>,
) -> Result<Json<PageView>, ApiError> {
    let provider = provider(&id)?;
    let query = filter.try_into()?;
    read(app, move |app| {
        app.inventory
            .project(app, provider, &key, |app, snapshot, project| {
                PageView::project(app, snapshot, project, &query)
            })
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

async fn claude_storage(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<agentmender_provider_claude::StorageDetail>, ApiError> {
    read(app, move |app| {
        let (session, evidence) = app.inventory.selected_claude(app, &key)?;
        app.claude
            .storage_detail(&session.identity, &evidence)
            .map_err(|_| {
                ApiError::conflict(
                    "Archivio Claude cambiato o non leggibile. Aggiorna l’inventario.",
                )
            })
    })
    .await
}

fn asset(content_type: &'static str, bytes: &'static str) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, content_type)], bytes)
}

fn router(app: Arc<App>) -> Router {
    let routes = Router::new();
    #[cfg(target_os = "macos")]
    let routes = routes.route(
        "/api/claude/sessions/{key}/removal-preview",
        post(removal::claude_preview),
    );
    routes
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
        .route("/api/storage/codex", get(storage::discover))
        .route("/api/storage/claude", get(claude_usage::inspect))
        .route("/api/storage/claude/refresh", post(claude_usage::refresh))
        .route("/api/storage/exports", get(storage::list))
        .route("/api/storage/exports/refresh", post(storage::refresh))
        .route("/api/storage/exports/preview", post(storage::preview))
        .route(
            "/api/storage/exports/operations/{key}/confirm",
            post(storage::confirm),
        )
        .route("/api/{provider}/projects", get(projects))
        .route("/api/{provider}/projects/{key}", get(project_detail))
        .route(
            "/api/{provider}/projects/{key}/sessions",
            get(project_sessions),
        )
        .route("/api/codex/projects/{key}/delete", post(removal::project))
        .route("/api/codex/recents", get(recents))
        .route("/api/{provider}/sessions", get(list))
        .route("/api/{provider}/sessions/{key}", get(detail))
        .route(
            "/api/{provider}/sessions/{key}/conversation",
            get(conversation),
        )
        .route(
            "/api/{provider}/sessions/{key}/delete",
            post(removal::session),
        )
        .route("/api/claude/sessions/{key}/storage", get(claude_storage))
        .route("/api/{provider}/refresh", post(refresh))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(app.clone(), boundary))
        .with_state(app)
}

pub(super) struct Configuration {
    pub codex_root: PathBuf,
    pub codex: CodexAdapter,
    pub discovery: Discovery,
    pub claude: ClaudeAdapter,
    pub limit: usize,
    pub protected: Vec<String>,
    pub no_open: bool,
}

pub(super) fn run(config: Configuration) -> Result<(), String> {
    let error = str::to_owned;
    if INDEX.is_empty() || SCRIPT.is_empty() || STYLES.is_empty() {
        return Err(error(
            "UI assets are missing. Run npm ci and npm run build in ui/, then rebuild AgentMender.",
        ));
    }
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
            codex_root: config.codex_root,
            codex: config.codex,
            discovery: config.discovery,
            exports: Mutex::default(),
            claude_usage: Mutex::default(),
            #[cfg(target_os = "macos")]
            claude_review: Mutex::default(),
            claude: config.claude,
            limit: config.limit,
            protected: config.protected,
            inventory: Inventories::default(),
            cursors: Mutex::default(),
            reads: Arc::new(Semaphore::new(8)),
        });
        println!("{url}");
        if !config.no_open && browser::open(addr, &launch).is_err() {
            eprintln!("Open the local URL printed above in your browser.");
        }
        axum::serve(listener, router(app))
            .await
            .map_err(|_| error("The local HTTP server stopped unexpectedly."))?;
        Ok(())
    })
}
