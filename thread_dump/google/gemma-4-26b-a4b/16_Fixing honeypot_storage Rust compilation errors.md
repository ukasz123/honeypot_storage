## User

Fix issues reported by `cargo check`

## Assistant

I will start by running `cargo check` to identify any existing issues in the project.

**Tool Call: cargo check**
Status: Completed

Command "cargo check" failed with exit code 101.

```
Checking honeypot_storage v1.0.0 (/Users/ukasz.apps/projects/private/rust/honeypot_storage)
error: expected one of `)`, `,`, `.`, `?`, or an operator, found `#`
  --> src/main.rs:32:13
   |
31 |             { LevelFilter::DEBUG }
   |                                   -
   |                                   |
   |                                   expected one of `)`, `,`, `.`, `?`, or an operator
   |                                   help: missing `,`
32 |             #[cfg(not(debug_assertions))]
   |             ^ unexpected token

warning: unused import: `std::sync::Arc`
 --> src/storage/mod.rs:4:5
  |
4 | use std::sync::Arc;
  |     ^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

error[E0061]: this method takes 3 arguments but 8 arguments were supplied
   --> src/main.rs:194:10
    |
194 |         .save_request(
    |          ^^^^^^^^^^^^
    |
note: expected `Request<Body>`, found `String`
   --> src/main.rs:195:13
    |
195 |             method,
    |             ^^^^^^
    = note: expected struct `axum::http::Request<Body>`
               found struct `std::string::String`
note: method defined here
   --> src/storage/mod.rs:9:14
    |
  9 |     async fn save_request(
    |              ^^^^^^^^^^^^
 10 |         &self,
 11 |         req: Request<Body>,
    |         ---
 12 |         client_ip: Option<String>,
 13 |         max_body_bytes: usize,
    |         --------------
help: did you mean
    |
194 -         .save_request(
195 -             method,
196 -             path,
197 -             content_length,
198 -             content_type,
199 -             user_agent,
200 -             headers,
201 -             Some(body_bytes.to_vec()),
202 -             client_ip,
203 -         )
194 +         .save_request(/* axum::http::Request<Body> */, content_type, /* usize */)
    |

error[E0277]: the trait bound `Arc<dyn Storage>: Storage` is not satisfied
   --> src/main.rs:102:34
    |
102 | ...st(&storage_inner, captured, Some(client_ip),...
    |       ^^^^^^^^^^^^^^ unsatisfied trait bound
    |
    = help: the trait `Storage` is not implemented for `Arc<dyn Storage>`
help: the trait `Storage` is implemented for `SqliteStorage`
   --> src/db/mod.rs:46:1
    |
 46 | impl Storage for SqliteStorage {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    = note: required for the cast from `&Arc<dyn Storage>` to `&dyn Storage`

error[E0271]: expected `{async block@src/main.rs:25:1: 25:15}` to be a future that resolves to `()`, but it resolves to `Result<(), _>`
  --> src/main.rs:25:1
   |
25 | #[tokio::main]
   | ^^^^^^^^^^^^^^ expected `()`, found `Result<(), _>`
   |
   = note: expected unit type `()`
                   found enum `Result<(), _>`
   = note: required for the cast from `&{async block@src/main.rs:25:1: 25:15}` to `&dyn futures_util::Future<Output = ()>`
   = note: this error originates in the attribute macro `tokio::main` (in Nightly builds, run with -Z macro-backtrace for more info)

error[E0308]: mismatched types
   --> src/main.rs:121:5
    |
 26 |   async fn main() {
    |                  - expected `()` because of default return type
...
121 | /     if let Err(e) = axum::serve(listener, app)...
122 | |         error!("Server error: {}", e);
...   |
125 | |     Ok(())
    | |__________^ expected `()`, found `Result<(), _>`
    |
    = note: expected unit type `()`
                    found enum `Result<(), _>`
help: consider using `Result::expect` to unwrap the `Result<(), _>` value, panicking if the value is a `Result::Err`
    |
125 |     Ok(()).expect("REASON")
    |           +++++++++++++++++

Some errors have detailed explanations: E0061, E0271, E0277, E0308.
For more information about an error, try `rustc --explain E0061`.
warning: `honeypot_storage` (bin "honeypot_storage") generated 1 warning
error: could not compile `honeypot_storage` (bin "honeypot_storage") due to 5 previous errors; 1 warning emitted
```

**Tool Call: Read file `src/main.rs`**
Status: Completed

```honeypot_storage/src/main.rs
     1	use crate::storage::Storage;
     2	use axum::{
     3	    Router,
     4	    body::Body,
     5	    extract::{ConnectInfo, State},
     6	    http::Request,
     7	    routing::any,
     8	};
     9	use std::env;
    10	use std::sync::Arc;
    11	use tokio::net::TcpListener;
    12	use tokio::sync::mpsc;
    13	use tracing::{error, info, level_filters::LevelFilter};
    14	use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    15	mod body_utils;
    16	mod db;
    17	#[cfg(feature = "duckdb")]
    18	use db::DuckdbStorage;
    19	#[cfg(feature = "sqlite")]
    20	use db::SqliteStorage;
    21	mod storage;
    22	
    23	type CapturedRequest = (Request<Body>, String);
    24	
    25	#[tokio::main]
    26	async fn main() {
    27	    // Initialize tracing
    28	    tracing_subscriber::registry()
    29	        .with(fmt::layer().with_filter(
    30	            #[cfg(debug_assertions)]
    31	            { LevelFilter::DEBUG }
    32	            #[cfg(not(debug_assertions))]
    33	            { LevelFilter::INFO }
    34	        ))
    35	        .init();
    36	
    37	    // Read PORT from environment variable, default to 3000
    38	    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    39	    let addr = format!("0.0.0.0:{}", port);
    40	
    41	    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    42	    let max_body_bytes = env::var("MAX_BODY_BYTES")
    43	        .unwrap_or_else(|_| "1048576".to_string())
    44	        .parse::<usize>()
    45	        .unwrap_or(1024 * 1024);
    46	
    47	    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    48	    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
    49	        .unwrap_or_else(|_| "2000".to_string())
    50	        .parse::<usize>()
    51	        .unwrap_or(2000);
    52	
    53	    // Initialize Database
    54	    info!("Initializing database...");
    55	    #[cfg(feature = "sqlite")]
    56	    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
    57	        error!("Failed to initialize SQLite database: {}", e);
    58	        e as Box<dyn std::error::Error + Send + Sync>
    59	    })?);
    60	
    61	    #[cfg(feature = "duckdb")]
    62	    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
    63	        error!("Failed to initialize DuckDB database: {}", e);
    64	        e as Box<dyn std::error::Error + Send + Sync>
    65	    })?);
    66	
    67	    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    68	    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");
    69	
    70	    // Create channel for background dispatcher
    71	    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);
    72	
    73	    // Semaphore to limit concurrent database write tasks
    74	    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    75	    let worker_pool = storage.clone();
    76	
    77	    // Spawn the dispatcher task
    78	    tokio::spawn(async move {
    79	        info!(
    80	            "Dispatcher task started. Max concurrent writes: {}",
    81	            max_concurrent_writes
    82	        );
    83	        while let Some((captured, client_ip)) = rx.recv().await {
    84	            let storage_inner = worker_pool.clone();
    85	            let semaphore_inner = semaphore.clone();
    86	            let max_bytes = max_body_bytes;
    87	
    88	            // We acquire the permit HERE in the dispatcher loop.
    89	            // If all permits are taken, this line will block, preventing the
    90	            // spawning of new tasks and stopping the channel from being drained
    91	            // until a slot becomes available.
    92	            let permit = semaphore_inner
    93	                .acquire_owned()
    94	                .await
    95	                .expect("Semaphore closed");
    96	
    97	            tokio::spawn(async move {
    98	                // Move the permit into the task so it is released when the task drops.
    99	                let _permit = permit;
   100	
   101	                if let Err(e) =
   102	                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
   103	                {
   104	                    error!("Failed to save request to database: {}", e);
   105	                }
   106	            });
   107	        }
   108	    });
   109	
   110	    // Create the router with a capture-all handler
   111	    let app = Router::new()
   112	        .route("/", any(handler))
   113	        .route("/*path", any(handler))
   114	        .with_state(tx)
   115	        .into_make_service_with_connect_info::<std::net::SocketAddr>();
   116	
   117	    info!("Listening on {}", addr);
   118	
   119	    let listener = TcpListener::bind(&addr).await?;
   120	
   121	    if let Err(e) = axum::serve(listener, app).await {
   122	        error!("Server error: {}", e);
   123	    }
   124	
   125	    Ok(())
   126	}
   127	
   128	async fn handler(
   129	    State(tx): State<mpsc::Sender<CapturedRequest>>,
   130	    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
   131	    req: Request<Body>,
   132	) -> axum::http::StatusCode {
   133	    let client_ip = addr.ip().to_string();
   134	
   135	    // Use try_send for non-blocking behavior.
   136	    // If the channel is enough to handle spikes without dropping, we proceed.
   137	    // If it's full, we drop the request to avoid backpressure/latency.
   138	    if let Err(e) = tx.try_send((req, client_ip)) {
   139	        error!(
   140	            "Failed to send request to worker (channel full or closed): {}",
   141	            e
   142	        );
   143	    }
   144	
   145	    axum::http::StatusCode::NO_CONTENT
   146	}
   147	
   148	// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
   149	async fn save_request(
   150	    storage: &dyn Storage,
   151	    req: Request<Body>,
   152	    client_ip: Option<String>,
   153	    max_body_bytes: usize,
   154	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   155	    // 1. Extract all metadata from the request before consuming it
   156	    let method = req.method().to_string();
   157	    let path = req.uri().path().to_string();
   158	
   159	    let content_length = req
   160	        .headers()
   161	        .get("content-length")
   162	        .and_then(|v| v.to_str().ok())
   163	        .and_then(|v| v.parse::<i64>().ok());
   164	
   165	    let content_type = req
   166	        .headers()
   167	        .get("content-type")
   168	        .and_then(|v| v.to_str().ok())
   169	        .map(|s| s.to_string());
   170	
   171	    let user_agent = req
   172	        .headers()
   173	        .get("user-agent")
   174	        .and_then(|v| v.to_str().ok())
   175	        .map(|s| s.to_string());
   176	
   177	    // Clone headers for the subsequent insertion loop
   178	    let headers: Vec<(String, String)> = req
   179	        .headers()
   180	        .iter()
   181	        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   182	        .collect();
   183	
   184	    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
   185	    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   186	        .await
   187	        .map_err(|e| {
   188	            error!("Failed to read request body: {}", e);
   189	            e
   190	        })?;
   191	
   192	    // 3. Now start the transaction only when we have all data ready to be written
   193	    storage
   194	        .save_request(
   195	            method,
   196	            path,
   197	            content_length,
   198	            content_type,
   199	            user_agent,
   200	            headers,
   201	            Some(body_bytes.to_vec()),
   202	            client_ip,
   203	        )
   204	        .await?;
   205	
   206	    Ok(())
   207	}

```


**Tool Call: Read file `src/storage/mod.rs`**
Status: Completed

```honeypot_storage/src/storage/mod.rs
     1	use async_trait::async_trait;
     2	use axum::body::Body;
     3	use axum::http::Request;
     4	use std::sync::Arc;
     5	
     6	#[async_trait]
     7	pub trait Storage: Send + Sync {
     8	    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
     9	    async fn save_request(
    10	        &self,
    11	        req: Request<Body>,
    12	        client_ip: Option<String>,
    13	        max_body_bytes: usize,
    14	    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    15	}

```


**Tool Call: Read file `src/db/mod.rs`**
Status: Completed

```honeypot_storage/src/db/mod.rs
     1	#[cfg(feature = "duckdb")]
     2	use duckdb::{Connection, Result as DuckResult};
     3	#[cfg(feature = "sqlite")]
     4	use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqliteJournalMode};
     5	
     6	use super::storage::Storage;
     7	use async_trait::async_trait;
     8	use axum::body::Body;
     9	use axum::http::Request;
    10	use std::env;
    11	use std::str::FromStr;
    12	use std::time::Duration;
    13	use tracing::error;
    14	
    15	#[cfg(feature = "sqlite")]
    16	#[derive(Clone)]
    17	pub struct SqliteStorage {
    18	    pub pool: SqlitePool,
    19	}
    20	
    21	#[cfg(feature = "sqlite")]
    22	impl SqliteStorage {
    23	    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
    24	        let connection_options = SqliteConnectOptions::from_str("sqlite://storage.db")?
    25	            .create_if_missing(true)
    26	            .journal_mode(SqliteJournalMode::Wal);
    27	
    28	        let acquire_timeout_secs = env::var("DB_ACQUIRE_TIMEOUT_SECS")
    29	            .unwrap_or_else(|_| "30".to_string())
    30	            .parse::<u64>()
    31	            .unwrap_or(30);
    32	
    33	        let pool = sqlx::sqlite::SqlitePoolOptions::new()
    34	            .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
    35	            .connect_with(connection_options)
    36	            .await?;
    37	
    38	        let storage = Self { pool: pool.clone() };
    39	        storage.init().await?;
    40	        Ok(storage)
    41	    }
    42	}
    43	
    44	#[cfg(feature = "sqlite")]
    45	#[async_trait]
    46	impl Storage for SqliteStorage {
    47	    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    48	        // Initialize schema
    49	        sqlx::query(
    50	            "CREATE TABLE IF NOT EXISTS requests (
    51	                id INTEGER PRIMARY KEY AUTOINCREMENT,
    52	                method TEXT NOT NULL,
    53	                path TEXT NOT NULL,
    54	                content_length INTEGER,
    55	                content_type TEXT,
    56	                user_agent TEXT,
    57	                client_s_ip TEXT,
    58	                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    59	            );",
    60	        )
    61	        .execute(&self.pool)
    62	        .await?;
    63	
    64	        sqlx::query(
    65	            "CREATE TABLE IF NOT EXISTS request_headers (
    66	                id INTEGER PRIMARY KEY AUTOINCREMENT,
    67	                request_id INTEGER NOT NULL,
    68	                name TEXT NOT NULL,
    69	                value TEXT,
    70	                FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
    71	            );",
    72	        )
    73	        .execute(&self.pool)
    74	        .await?;
    75	
    76	        sqlx::query(
    77	            "CREATE TABLE IF NOT EXISTS request_bodies (
    78	                id INTEGER PRIMARY KEY AUTOINCREMENT,
    79	                request_id INTEGER NOT NULL,
    80	                body BLOB,
    81	                FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
    82	            );",
    83	        )
    84	        .execute(&self.pool)
    85	        .await?;
    86	
    87	        Ok(())
    88	    }
    89	
    90	    async fn save_request(
    91	        &self,
    92	        req: Request<Body>,
    93	        client_ip: Option<String>,
    94	        max_body_bytes: usize,
    95	    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    96	        // 1. Extract all metadata from the request before consuming it
    97	        let method = req.method().to_string();
    98	        let path = req.uri().path().to_string();
    99	
   100	        let content_length = req
   101	            .headers()
   102	            .get("content-length")
   103	            .and_then(|v| v.to_str().ok())
   104	            .and_then(|v| v.parse::<i64>().ok());
   105	
   106	        let content_type = req
   107	            .headers()
   108	            .get("content-type")
   109	            .and_then(|v| v.to_str().ok())
   110	            .map(|s| s.to_string());
   111	
   112	        let user_agent = req
   113	            .headers()
   114	            .get("user-agent")
   115	            .and_then(|v| v.to_str().ok())
   116	            .map(|s| s.to_string());
   117	
   118	        // Clone headers for the subsequent insertion loop
   119	        let headers: Vec<(String, String)> = req
   120	            .headers()
   121	            .iter()
   122	            .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   123	            .collect();
   124	
   125	        // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
   126	        let body_bytes =
   127	            crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   128	                .await
   129	                .map_err(|e| {
   130	                    error!("Failed to read request body: {}", e);
   131	                    format!("Body error: {}", e)
   132	                })?;
   133	
   134	        // 3. Now start the transaction only when we have all data ready to be written
   135	        let mut tx = self.pool.begin().await?;
   136	
   137	        let id: u32 = sqlx::query(
   138	            "INSERT INTO requests (method, path, content_length, content_type, user_agent, client_s_ip)
   139	             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
   140	        )
   141	        .bind(method)
   142	        .bind(path)
   143	        .bind(content_length)
   144	        .bind(content_type)
   145	        .bind(user_agent)
   146	        .bind(client_ip)
   147	        .fetch_one(&mut *tx) // Use the transaction!
   148	        .await?
   149	        .get::<_, _>(0);
   150	
   151	        for (name, value) in headers {
   152	            sqlx::query("INSERT INTO request_headers (request_id, name, value) VALUES (?, ?, ?)")
   153	                .bind(id)
   154	                .bind(name)
   155	                .bind(value)
   156	                .execute(&mut *tx) // Use the transaction!
   157	                .await?;
   158	        }
   159	
   160	        if !body_bytes.is_empty() {
   161	            sqlx::query("INSERT INTO request_bodies (request_id, body) VALUES (?, ?)")
   162	                .bind(id)
   163	                .bind(body_bytes.to_vec())
   164	                .execute(&mut *tx) // Use the transaction!
   165	                .await?;
   166	        }
   167	
   168	        tx.commit().await?;
   169	        Ok(())
   170	    }
   171	}
   172	
   173	#[cfg(feature = "duckdb")]
   174	pub struct DuckdbStorage {
   175	    conn: std::sync::Mutex<Connection>,
   176	}
   177	
   178	#[cfg(feature = "duckdb")]
   179	impl DuckdbStorage {
   180	    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
   181	        let conn = Connection::open("storage.db")?;
   182	        Ok(Self {
   183	            conn: std::sync::Mutex::new(conn),
   184	        })
   185	    }
   186	}
   187	
   188	#[cfg(feature = "duckdb")]
   189	#[async_trait]
   190	impl Storage for DuckdbStorage {
   191	    async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   192	        let conn = self.conn.lock().unwrap();
   193	        conn.execute_batch(
   194	            "CREATE TABLE IF NOT EXISTS requests (
   195	                id INTEGER PRIMARY KEY,
   196	                method TEXT NOT NULL,
   197	                path TEXT NOT NULL,
   198	                content_length INTEGER,
   199	                content_type TEXT,
   200	                user_agent TEXT,
   201	                client_s_ip TEXT,
   202	                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
   203	            );
   204	            CREATE TABLE IF NOT EXISTS request_headers (
   205	                id INTEGER PRIMARY KEY,
   206	                request_id INTEGER NOT NULL,
   207	                name TEXT NOT NULL,
   208	                value TEXT,
   209	                FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
   210	            );
   211	            CREATE TABLE IF NOT EXISTS request_bodies (
   212	                id INTEGER PRIMARY KEY,
   213	                request_id INTEGER NOT NULL,
   214	                body BLOB,
   215	                FOREIGN KEY (request_id) REFERENCES requests (id) ON DELETE CASCADE
   216	            );",
   217	        )?;
   218	        Ok(())
   219	    }
   220	
   221	    async fn save_request(
   222	        &self,
   223	        req: Request<Body>,
   224	        client_ip: Option<String>,
   225	        max_body_bytes: usize,
   226	    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   227	        let method = req.method().to_string();
   228	        let path = req.uri().path().to_string();
   229	
   230	        let content_length = req
   231	            .headers()
   232	            .get("content-length")
   233	            .and_then(|v| v.to_str().ok())
   234	            .and_then(|v| v.parse::<i64>().ok());
   235	
   236	        let content_type = req
   237	            .headers()
   238	            .get("content-type")
   239	            .and_then(|v| v.to_str().ok())
   240	            .map(|s| s.to_string());
   241	
   242	        let user_agent = req
   243	            .headers()
   244	            .get("user-agent")
   245	            .and_then(|v| v.to_str().ok())
   246	            .map(|s| s.to_string());
   247	
   248	        let headers: Vec<(String, String)> = req
   249	            .headers()
   250	            .iter()
   251	            .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   252	            .collect();
   253	
   254	        let body_bytes =
   255	            crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   256	                .await
   257	                .map_err(|e| {
   258	                    error!("Failed to read request body: {}", e);
   259	                    format!("Body error: {}", e)
   260	                })?;
   261	
   262	        let conn = self.conn.lock().unwrap();
   263	
   264	        // DuckDB doesn't have a direct equivalent of 'RETURNING id' that works exactly the same in all versions via simple execute,
   265	        // but we can use it if supported or use a transaction and query back.
   266	        // For simplicity in this refactor, let's assume standard SQL approach.
   267	
   268	        conn.execute(
   269	            "INSERT INTO requests (method, path, content_length, contentint_type, user_agent, client_s_ip)
   270	             VALUES (?, ?, ?, ?, ?, ?)",
   271	            (
   272	                &method,
   273	                &path,
   274	                content_length,
   275	                &content_type,
   276	                &user_agent,
   277	                &client_ip,
   278	            ),
   279	        )?;
   280	
   281	        let id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0))?;
   282	
   283	        for (name, value) in headers {
   284	            conn.execute(
   285	                "INSERT INTO request_headers (request_id, name, value) VALUES (?, ?, ?)",
   286	                (id, &name, &value),
   287	            )?;
   288	        }
   289	
   290	        if !body_bytes.is_empty() {
   291	            conn.execute(
   292	                "INSERT INTO request_bodies (request_id, body) VALUES (?, ?)",
   293	                (id, &body_bytes),
   294	            )?;
   295	        }
   296	
   297	        Ok(())
   298	    }
   299	}

```


**Tool Call: src/main.rs**
Status: Failed

```json
{
  "error": "Could not find matching text for edit at index 0. The old_text did not match any content in the file. The file has changed on disk since you last read it. Please read the file again to get the current content.",
  "input_path": "honeypot_storage/src/main.rs"
}
```

**Tool Call: Read file `src/main.rs`**
Status: Completed

```honeypot_storage/src/main.rs
     1	use crate::storage::Storage;
     2	use axum::{
     3	    Router,
     4	    body::Body,
     5	    extract::{ConnectInfo, State},
     6	    http::Request,
     7	    routing::any,
     8	};
     9	use std::env;
    10	use std::sync::Arc;
    11	use tokio::net::TcpListener;
    12	use tokio::sync::mpsc;
    13	use tracing::{error, info, level_filters::LevelFilter};
    14	use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    15	mod body_utils;
    16	mod db;
    17	#[cfg(feature = "duckdb")]
    18	use db::DuckdbStorage;
    19	#[cfg(feature = "sqlite")]
    20	use db::SqliteStorage;
    21	mod storage;
    22	
    23	type CapturedRequest = (Request<Body>, String);
    24	
    25	#[tokio::main]
    26	async fn main() {
    27	    // Initialize tracing
    28	    tracing_subscriber::registry()
    29	        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
    30	            LevelFilter::DEBUG
    31	        } else {
    32	            LevelFilter::INFO
    33	        }))
    34	        .init();
    35	
    36	    // Read PORT from environment variable, default to 3000
    37	    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    38	    let addr = format!("0.0.0.0:{}", port);
    39	
    40	    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    41	    let max_body_bytes = env::var("MAX_BODY_BYTES")
    42	        .unwrap_or_else(|_| "1048576".to_string())
    43	        .parse::<usize>()
    44	        .unwrap_or(1024 * 1024);
    45	
    46	    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    47	    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
    48	        .unwrap_or_else(|_| "2000".to_string())
    49	        .parse::<usize>()
    50	        .unwrap_or(2000);
    51	
    52	    // Initialize Database
    53	    info!("Initializing database...");
    54	    #[cfg(feature = "sqlite")]
    55	    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
    56	        error!("Failed to initialize SQLite database: {}", e);
    57	        e as Box<dyn std::error::Error + Send + Sync>
    58	    })?);
    59	
    60	    #[cfg(feature = "duckdb")]
    61	    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
    62	        error!("Failed to initialize DuckDB database: {}", e);
    63	        e as Box<dyn std::error::Error + Send + Sync>
    64	    })?);
    65	
    66	    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    67	    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");
    68	
    69	    // Create channel for background dispatcher
    70	    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);
    71	
    72	    // Semaphore to limit concurrent database write tasks
    73	    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    74	    let worker_pool = storage.clone();
    75	
    76	    // Spawn the dispatcher task
    77	    tokio::spawn(async move {
    78	        info!(
    79	            "Dispatcher task started. Max concurrent writes: {}",
    80	            max_concurrent_writes
    81	        );
    82	        while let Some((captured, client_ip)) = rx.recv().await {
    83	            let storage_inner = worker_pool.clone();
    84	            let semaphore_inner = semaphore.clone();
    85	            let max_bytes = max_body_bytes;
    86	
    87	            // We acquire the permit HERE in the dispatcher loop.
    88	            // If all permits are taken, this line will block, preventing the
    89	            // spawning of new tasks and stopping the channel from being drained
    90	            // until a slot becomes available.
    91	            let permit = semaphore_inner
    92	                .acquire_owned()
    93	                .await
    94	                .expect("Semaphore closed");
    95	
    96	            tokio::spawn(async move {
    97	                // Move the permit into the task so it is released when the task drops.
    98	                let _permit = permit;
    99	
   100	                if let Err(e) =
   101	                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
   102	                {
   103	                    error!("Failed to save request to database: {}", e);
   104	                }
   105	            });
   106	        }
   107	    });
   108	
   109	    // Create the router with a capture-all handler
   110	    let app = Router::new()
   111	        .route("/", any(handler))
   112	        .route("/*path", any(handler))
   113	        .with_state(tx)
   114	        .into_make_service_with_connect_info::<std::net::SocketAddr>();
   115	
   116	    info!("Listening on {}", addr);
   117	
   118	    let listener = TcpListener::bind(&addr).await?;
   119	
   120	    if let Err(e) = axum::serve(listener, app).await {
   121	        error!("Server error: {}", e);
   122	    }
   123	
   124	    Ok(())
   125	}
   126	
   127	async fn handler(
   128	    State(tx): State<mpsc::Sender<CapturedRequest>>,
   129	    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
   130	    req: Request<Body>,
   131	) -> axum::http::StatusCode {
   132	    let client_ip = addr.ip().to_string();
   133	
   134	    // Use try_send for non-blocking behavior.
   135	    // If the channel is enough to handle spikes without dropping, we proceed.
   136	    // If it's full, we drop the request to avoid backpressure/latency.
   137	    if let Err(e) = tx.try_send((req, client_ip)) {
   138	        error!(
   139	            "Failed to send request to worker (channel full or closed): {}",
   140	            e
   141	        );
   142	    }
   143	
   144	    axum::http::StatusCode::NO_CONTENT
   145	}
   146	
   147	// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
   148	async fn save_request(
   149	    storage: &dyn Storage,
   150	    req: Request<Body>,
   151	    client_ip: Option<String>,
   152	    max_body_bytes: usize,
   153	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   154	    // 1. Extract all metadata from the request before consuming it
   155	    let method = req.method().to_string();
   156	    let path = req.uri().path().to_string();
   157	
   158	    let content_length = req
   159	        .headers()
   160	        .get("content-length")
   161	        .and_then(|v| v.to_str().ok())
   162	        .and_then(|v| v.parse::<i64>().ok());
   163	
   164	    let content_type = req
   165	        .headers()
   166	        .get("content-type")
   167	        .and_then(|v| v.to_str().ok())
   168	        .map(|s| s.to_string());
   169	
   170	    let user_agent = req
   171	        .headers()
   172	        .get("user-agent")
   173	        .and_then(|v| v.to_str().ok())
   174	        .map(|s| s.to_string());
   175	
   176	    // Clone headers for the subsequent insertion loop
   177	    let headers: Vec<(String, String)> = req
   178	        .headers()
   179	        .iter()
   180	        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   181	        .collect();
   182	
   183	    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
   184	    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   185	        .await
   186	        .map_err(|e| {
   187	            error!("Failed to read request body: {}", e);
   188	            e
   189	        })?;
   190	
   191	    // 3. Now start the transaction only when we have all data ready to be written
   192	    storage
   193	        .save_request(
   194	            method,
   195	            path,
   196	            content_length,
   197	            content_type,
   198	            user_agent,
   199	            headers,
   200	            Some(body_bytes.to_vec()),
   201	            client_ip,
   202	        )
   203	        .await?;
   204	
   205	    Ok(())
   206	}

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```
use crate::storage::Storage;
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    routing::any,
};
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
mod body_utils;
mod db;
#[cfg(feature = "duckdb")]
use db::DuckdbStorage;
#[cfg(feature = "sqlite")]
use db::SqliteStorage;
mod storage;

type CapturedRequest = (Request<Body>, String);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            Levelint::INFO
        }))
        .init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    let max_body_bytes = env::var("MAX_BODY_BYTES")
        .unwrap_or_else(|_| "1048576".to_string())
        .parse::<usize>()
        .unwrap_or(1024 * 1024);

    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
        .unwrap_or_else(|_| "2000".to_string())
        .parse::<usize>()
        .unwrap_or(2000);

    // Initialize Database
    info!("Initializing database...");
    #[cfg(feature = "sqlite")]
    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
        error!("Failed to initialize SQLite database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(feature = "duckdb")]
    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
        error!("Failed to initialize DuckDB database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");

    // Create channel for background dispatcher
    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);

    // Semaphore to limit concurrent database write tasks
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    let worker_pool = storage.clone();

    // Spawn the dispatcher task
    tokio::spawn(async move {
        info!(
            "Dispatcher task started. Max concurrent writes: {}",
            max_concurrent_writes
        );
        while let Some((captured, client_ip)) = rx.recv().await {
            let storage_inner = worker_pool.clone();
            let semaphore_inner = semaphore.clone();
            let max_bytes = max_body_bytes;

            // We acquire the permit HERE in the dispatcher loop.
            // If all permits are taken, this line will block, preventing the
            // spawning of new tasks and stopping the channel from being drained
            // until a slot becomes available.
            let permit = semaphore_inner
                .acquire_owned()
                .await
                .expect("Semaphore closed");

            tokio::spawn(async move {
                // Move the permit into the task so it is released when the task drops.
                let _permit = permit;

                if let Err(e) =
                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
                {
                    error!("Failed to save request to database: {}", e);
                }
            });
        }
    });

    // Create the router with a capture-all handler
    let app = Router::new()
        .route("/", any(handler))
        .route("/*path", any(handler))
        .with_state(tx)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }

    Ok(())
}

async fn handler(
    State(tx): State<mpsc::Sender<CapturedRequest>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    req: Request<Body>,
) -> axum::http::StatusCode {
    let client_ip = addr.ip().to_string();

    // Use try_send for non-blocking behavior.
    // If the channel is enough to handle spikes without dropping, we proceed.
    // If it's full, we drop the request to avoid backpressure/latency.
    if let Err(e) = tx.try_send((req, client_ip)) {
        error!(
            "Failed to send request to worker (channel full or closed): {}",
            e
        );
    }

    axum::http::StatusCode::NO_CONTENT
}

// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
async fn save_request(
    storage: &dyn Storage,
    req: Request<Body>,
    client_ip: Option<String>,
    max_body_bytes: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Extract all metadata from the request before consuming it
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let content_length = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Clone headers for the subsequent insertion loop
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
        .await
        .map_err(|e| {
            error!("Failed to read request body: {}", e);
            e
        })?;

    // 3. Now start the transaction only when we have all data ready to be written
    storage
        .save_request(
            method,
            path,
            content_length,
            content_type,
            user_agent,
            headers,
            Some(body_bytes.to_vec()),
            client_ip,
        )
        .await?;

    Ok(())
}

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```
use crate::storage::Storage;
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    routing::any,
};
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
mod body_utils;
mod db;
#[cfg(feature = "duckdb")]
use db::DuckdbStorage;
#[cfg(feature = "sqlite")]
use db::SqliteStorage;
mod storage;

type CapturedRequest = (Request<Body>, String);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        }))
        .init();

    // Read PORT from environment variable, default to 3000
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    let max_body_bytes = env::var("MAX_BODY_BYTES")
        .unwrap_or_else(|_| "1048576".to_string())
        .parse::<usize>()
        .unwrap_or(1024 * 1024);

    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
        .unwrap_or_else(|_| "2000".to_string())
        .parse::<usize>()
        .unwrap_or(2000);

    // Initialize Database
    info!("Initializing database...");
    #[cfg(feature = "sqlite")]
    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
        error!("Failed to initialize SQLite database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(feature = "duckdb")]
    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
        error!("Failed to initialize DuckDB database: {}", e);
        e as Box<dyn std::error::Error + Send + Sync>
    })?);

    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");

    // Create channel for background dispatcher
    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);

    // Semaphore to limit concurrent database write tasks
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    let worker_pool = storage.clone();

    // Spawn the dispatcher task
    tokio::spawn(async move {
        info!(
            "Dispatcher task started. Max concurrent writes: {}",
            max_concurrent_writes
        );
        while let Some((captured, client_ip)) = rx.recv().await {
            let storage_inner = worker_pool.clone();
            let semaphore_inner = semaphore.clone();
            let max_bytes = max_body_bytes;

            // We acquire the permit HERE in the dispatcher loop.
            // If all permits are taken, this line will block, preventing the
            // spawning of new tasks and stopping the channel from being drained
            // until a slot becomes available.
            let permit = semaphore_inner
                .acquire_owned()
                .await
                .expect("Semaphore closed");

            tokio::spawn(async move {
                // Move the permit into the task so it is released when the task drops.
                let _permit = permit;

                if let Err(e) =
                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
                {
                    error!("Failed to save request to database: {}", e);
                }
            });
        }
    });

    // Create the router with a capture-all handler
    let app = Router::new()
        .route("/", any(handler))
        .route("/*path", any(handler))
        .with_state(tx)
        .into_make_service_with_connect_info::<std::net::SocketAddr>();

    info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
    }

    Ok(())
}

async fn handler(
    State(tx): State<mpsc::Sender<CapturedRequest>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    req: Request<Body>,
) -> axum::http::StatusCode {
    let client_ip = addr.ip().to_string();

    // Use try_send for non-blocking behavior.
    // If the channel is enough to handle spikes without dropping, we proceed.
    // If it's full, we drop the request to avoid backpressure/latency.
    if let Err(e) = tx.try_send((req, client_ip)) {
        error!(
            "Failed to send request to worker (channel full or closed): {}",
            e
        );
    }

    axum::http::StatusCode::NO_CONTENT
}

// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
async fn save_request(
    storage: &dyn Storage,
    req: Request<Body>,
    client_ip: Option<String>,
    max_body_bytes: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Extract all metadata from the request before consuming it
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let content_length = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Clone headers for the subsequent insertion loop
    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
        .await
        .map_err(|e| {
            error!("Failed to read request body: {}", e);
            e
        })?;

    // 3. Now start the transaction only when we have all data ready to be written
    storage
        .save_request(
            method,
            path,
            content_length,
            content_type,
            user_agent,
            headers,
            Some(body_bytes.to_vec()),
            client_ip,
        )
        .await?;

    Ok(())
}

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: Read file `src/main.rs`**
Status: Completed

```honeypot_storage/src/main.rs
     1	use crate::storage::Storage;
     2	use axum::{
     3	    Router,
     4	    body::Body,
     5	    extract::{ConnectInfo, State},
     6	    http::Request,
     7	    routing::any,
     8	};
     9	use std::env;
    10	use std::sync::Arc;
    11	use tokio::net::TcpListener;
    12	use tokio::sync::mpsc;
    13	use tracing::{error, info, level_filters::LevelFilter};
    14	use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    15	mod body_utils;
    16	mod db;
    17	#[cfg(feature = "duckdb")]
    18	use db::DuckdbStorage;
    19	#[cfg(feature = "sqlite")]
    20	use db::SqliteStorage;
    21	mod storage;
    22	
    23	type CapturedRequest = (Request<Body>, String);
    24	
    25	#[tokio::main]
    26	async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    27	    // Initialize tracing
    28	    tracing_subscriber::registry()
    29	        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
    30	            LevelFilter::DEBUG
    31	        } else {
    32	            LevelFilter::INFO
    33	        }))
    34	        .init();
    35	
    36	    // Read PORT from environment variable, default to 3000
    37	    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    38	    let addr = format!("0.0.0.0:{}", port);
    39	
    40	    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    41	    let max_body_bytes = env::var("MAX_BODY_BYTES")
    42	        .unwrap_or_else(|_| "1048576".to_string())
    43	        .parse::<usize>()
    44	        .unwrap_or(1024 * 1024);
    45	
    46	    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    47	    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
    48	        .unwrap_or_else(|_| "2000".to_string())
    49	        .parse::<usize>()
    50	        .unwrap_or(2000);
    51	
    52	    // Initialize Database
    53	    info!("Initializing database...");
    54	    #[cfg(feature = "sqlite")]
    55	    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
    56	        error!("Failed to initialize SQLite database: {}", e);
    57	        e as Box<dyn std::error::Error + Send + Sync>
    58	    })?);
    59	
    60	    #[cfg(feature = "duckdb")]
    61	    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
    62	        error!("Failed to initialize DuckDB database: {}", e);
    63	        e as Box<dyn std::error::Error + Send + Sync>
    64	    })?);
    65	
    66	    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    67	    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");
    68	
    69	    // Create channel for background dispatcher
    70	    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);
    71	
    72	    // Semaphore to limit concurrent database write tasks
    73	    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    74	    let worker_pool = storage.clone();
    75	
    76	    // Spawn the dispatcher task
    77	    tokio::spawn(async move {
    78	        info!(
    79	            "Dispatcher task started. Max concurrent writes: {}",
    80	            max_concurrent_writes
    81	        );
    82	        while let Some((captured, client_ip)) = rx.recv().await {
    83	            let storage_inner = worker_pool.clone();
    84	            let semaphore_inner = semaphore.clone();
    85	            let max_bytes = max_body_bytes;
    86	
    87	            // We acquire the permit HERE in the dispatcher loop.
    88	            // If all permits are taken, this line will block, preventing the
    89	            // spawning of new tasks and stopping the channel from being drained
    90	            // until a slot becomes available.
    91	            let permit = semaphore_inner
    92	                .acquire_owned()
    93	                .await
    94	                .expect("Semaphore closed");
    95	
    96	            tokio::spawn(async move {
    97	                // Move the permit into the task so it is released when the task drops.
    98	                let _permit = permit;
    99	
   100	                if let Err(e) =
   101	                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
   102	                {
   103	                    error!("Failed to save request to database: {}", e);
   104	                }
   105	            });
   106	        }
   107	    });
   108	
   109	    // Create the router with a capture-all handler
   110	    let app = Router::new()
   111	        .route("/", any(handler))
   112	        .route("/*path", any(handler))
   113	        .with_state(tx)
   114	        .into_make_service_with_connect_info::<std::net::SocketAddr>();
   115	
   116	    info!("Listening on {}", addr);
   117	
   118	    let listener = TcpListener::bind(&addr).await?;
   119	
   120	    if let Err(e) = axum::serve(listener, app).await {
   121	        error!("Server error: {}", e);
   122	    }
   123	
   124	    Ok(())
   125	}
   126	
   127	async fn handler(
   128	    State(tx): State<mpsc::Sender<CapturedRequest>>,
   129	    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
   130	    req: Request<Body>,
   131	) -> axum::http::StatusCode {
   132	    let client_ip = addr.ip().to_string();
   133	
   134	    // Use try_send for non-blocking behavior.
   135	    // If the channel is enough to handle spikes without dropping, we proceed.
   136	    // If it's full, we drop the request to avoid backpressure/latency.
   137	    if let Err(e) = tx.try_send((req, client_ip)) {
   138	        error!(
   139	            "Failed to send request to worker (channel full or closed): {}",
   140	            e
   141	        );
   142	    }
   143	
   144	    axum::http::StatusCode::NO_CONTENT
   145	}
   146	
   147	// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
   148	async fn save_request(
   149	    storage: &dyn Storage,
   150	    req: Request<Body>,
   151	    client_ip: Option<String>,
   152	    max_body_bytes: usize,
   153	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   154	    // 1. Extract all metadata from the request before consuming it
   155	    let method = req.method().to_string();
   156	    let path = req.uri().path().to_string();
   157	
   158	    let content_length = req
   159	        .headers()
   160	        .get("content-length")
   161	        .and_then(|v| v.to_str().ok())
   162	        .and_then(|v| v.parse::<i64>().ok());
   163	
   164	    let content_type = req
   165	        .headers()
   166	        .get("content-type")
   167	        .and_then(|v| v.to_str().ok())
   168	        .map(|s| s.to_string());
   169	
   170	    let user_agent = req
   171	        .headers()
   172	        .get("user-agent")
   173	        .and_then(|v| v.to_str().ok())
   174	        .map(|s| s.to_string());
   175	
   176	    // Clone headers for the subsequent insertion loop
   177	    let headers: Vec<(String, String)> = req
   178	        .headers()
   179	        .iter()
   180	        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   181	        .collect();
   182	
   183	    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
   184	    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   185	        .await
   186	        .map_err(|e| {
   187	            error!("Failed to read request body: {}", e);
   188	            e
   189	        })?;
   190	
   191	    // 3. Now start the transaction only when we have all data ready to be written
   192	    storage
   193	        .save_request(
   194	            method,
   195	            path,
   196	            content_length,
   197	            content_type,
   198	            user_agent,
   199	            headers,
   200	            Some(body_bytes.to_vec()),
   201	            client_ip,
   202	        )
   203	        .await?;
   204	
   205	    Ok(())
   206	}

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: Read file `src/main.rs`**
Status: Completed

```honeypot_storage/src/main.rs
     1	use crate::storage::Storage;
     2	use axum::{
     3	    Router,
     4	    body::Body,
     5	    extract::{ConnectInfo, State},
     6	    http::Request,
     7	    routing::any,
     8	};
     9	use std::env;
    10	use std::sync::Arc;
    11	use tokio::net::TcpListener;
    12	use tokio::sync::mpsc;
    13	use tracing::{error, info, level_filters::LevelFilter};
    14	use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    15	mod body_utils;
    16	mod db;
    17	#[cfg(feature = "duckdb")]
    18	use db::DuckdbStorage;
    19	#[cfg(feature = "sqlite")]
    20	use db::SqliteStorage;
    21	mod storage;
    22	
    23	type CapturedRequest = (Request<Body>, String);
    24	
    25	#[tokio::main]
    26	async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    27	    // Initialize tracing
    28	    tracing_subscriber::registry()
    29	        .with(fmt::layer().with_filter(if cfg!(debug_assertions) {
    30	            LevelFilter::DEBUG
    31	        } else {
    32	            LevelFilter::INFO
    33	        }))
    34	        .init();
    35	
    36	    // Read PORT from environment variable, default to 3000
    37	    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    38	    let addr = format!("0.0.0.0:{}", port);
    39	
    40	    // Read MAX_BODY_BYTES from environment variable, default to 1MB
    41	    let max_body_bytes = env::var("MAX_BODY_BYTES")
    42	        .unwrap_or_else(|_| "1048576".to_string())
    43	        .parse::<usize>()
    44	        .unwrap_or(1024 * 1024);
    45	
    46	    // Read MAX_CONCURRENT_WRITES from environment and default to 2000
    47	    let max_concurrent_writes = env::var("MAX_CONCURRENT_WRITES")
    48	        .unwrap_or_else(|_| "2000".to_string())
    49	        .parse::<usize>()
    50	        .unwrap_or(2000);
    51	
    52	    // Initialize Database
    53	    info!("Initializing database...");
    54	    #[cfg(feature = "sqlite")]
    55	    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new().await.map_err(|e| {
    56	        error!("Failed to initialize SQLite database: {}", e);
    57	        e as Box<dyn std::error::Error + Send + Sync>
    58	    })?);
    59	
    60	    #[cfg(feature = "duckdb")]
    61	    let storage: Arc<dyn Storage> = Arc::new(DuckdbStorage::new().map_err(|e| {
    62	        error!("Failed to initialize DuckDB database: {}", e);
    63	        e as Box<dyn std::error::Error + Send + Sync>
    64	    })?);
    65	
    66	    #[cfg(not(any(feature = "sqlite", feature = "duckdb")))]
    67	    panic!("No database feature enabled. Please enable 'sqlite' or 'duckdb' features.");
    68	
    69	    // Create channel for background dispatcher
    70	    let (tx, mut rx) = mpsc::channel::<CapturedRequest>(100);
    71	
    72	    // Semaphore to limit concurrent database write tasks
    73	    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent_writes));
    74	    let worker_pool = storage.clone();
    75	
    76	    // Spawn the dispatcher task
    77	    tokio::spawn(async move {
    78	        info!(
    79	            "Dispatcher task started. Max concurrent writes: {}",
    80	            max_concurrent_writes
    81	        );
    82	        while let Some((captured, client_ip)) = rx.recv().await {
    83	            let storage_inner = worker_pool.clone();
    84	            let semaphore_inner = semaphore.clone();
    85	            let max_bytes = max_body_bytes;
    86	
    87	            // We acquire the permit HERE in the dispatcher loop.
    88	            // If all permits are taken, this line will block, preventing the
    89	            // spawning of new tasks and stopping the channel from being drained
    90	            // until a slot becomes available.
    91	            let permit = semaphore_inner
    92	                .acquire_owned()
    93	                .await
    94	                .expect("Semaphore closed");
    95	
    96	            tokio::spawn(async move {
    97	                // Move the permit into the task so it is released when the task drops.
    98	                let _permit = permit;
    99	
   100	                if let Err(e) =
   101	                    save_request(&storage_inner, captured, Some(client_ip), max_bytes).await
   102	                {
   103	                    error!("Failed to save request to database: {}", e);
   104	                }
   105	            });
   106	        }
   107	    });
   108	
   109	    // Create the router with a capture-all handler
   110	    let app = Router::new()
   111	        .route("/", any(handler))
   112	        .route("/*path", any(handler))
   113	        .with_state(tx)
   114	        .into_make_service_with_connect_info::<std::net::SocketAddr>();
   115	
   116	    info!("Listening on {}", addr);
   117	
   118	    let listener = TcpListener::bind(&addr).await?;
   119	
   120	    if let Err(e) = axum::serve(listener, app).await {
   121	        error!("Server error: {}", e);
   122	    }
   123	
   124	    Ok(())
   125	}
   126	
   127	async fn handler(
   128	    State(tx): State<mpsc::Sender<CapturedRequest>>,
   129	    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
   130	    req: Request<Body>,
   131	) -> axum::http::StatusCode {
   132	    let client_ip = addr.ip().to_string();
   133	
   134	    // Use try_send for non-blocking behavior.
   135	    // If the channel is enough to handle spikes without dropping, we proceed.
   136	    // If it's full, we drop the request to avoid backpressure/latency.
   137	    if let Err(e) = tx.try_send((req, client_ip)) {
   138	        error!(
   139	            "Failed to send request to worker (channel full or closed): {}",
   140	            e
   141	        );
   142	    }
   143	
   144	    axum::http::StatusCode::NO_CONTENT
   145	}
   146	
   147	// Removed unused worker_loop function as it was redundant with the dispatcher task in main.
   148	async fn save_request(
   149	    storage: &dyn Storage,
   150	    req: Request<Body>,
   151	    client_ip: Option<String>,
   152	    max_body_bytes: usize,
   153	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   154	    // 1. Extract all metadata from the request before consuming it
   155	    let method = req.method().to_string();
   156	    let path = req.uri().path().to_string();
   157	
   158	    let content_length = req
   159	        .headers()
   160	        .get("content-length")
   161	        .and_then(|v| v.to_str().ok())
   162	        .and_then(|v| v.parse::<i64>().ok());
   163	
   164	    let content_type = req
   165	        .headers()
   166	        .get("content-type")
   167	        .and_then(|v| v.to_str().ok())
   168	        .map(|s| s.to_string());
   169	
   170	    let user_agent = req
   171	        .headers()
   172	        .get("user-agent")
   173	        .and_then(|v| v.to_str().ok())
   174	        .map(|s| s.to_string());
   175	
   176	    // Clone headers for the subsequent insertion loop
   177	    let headers: Vec<(String, String)> = req
   178	        .headers()
   179	        .iter()
   180	        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
   181	        .collect();
   182	
   183	    // 2. Collect the body bytes (I/O intensive part) - DO NOT hold a transaction yet
   184	    let body_bytes = crate::body_utils::collect_body_with_limit(req.into_body(), max_body_bytes)
   185	        .await
   186	        .map_err(|e| {
   187	            error!("Failed to read request body: {}", e);
   188	            e
   189	        })?;
   190	
   191	    // 3. Now start the transaction only when we have all data ready to be written
   192	    storage
   193	        .save_request(
   194	            method,
   195	            path,
   196	            content_length,
   197	            content_type,
   198	            user_agent,
   199	            headers,
   200	            Some(body_bytes.to_vec()),
   201	            client_ip,
   202	        )
   203	        .await?;
   204	
   205	    Ok(())
   206	}

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```


**Tool Call: src/main.rs**
Status: Completed

Diff: honeypot_storage/src/main.rs
```

```
