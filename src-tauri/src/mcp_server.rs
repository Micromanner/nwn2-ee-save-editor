use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use tokio::sync::{Mutex, mpsc::UnboundedSender};
use tokio_stream::{StreamExt as _, wrappers::UnboundedReceiverStream};

use crate::loaders::types::GameData;
use crate::state::session_state::SessionState;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

type Sessions = Arc<Mutex<HashMap<String, UnboundedSender<String>>>>;

#[derive(Clone)]
pub struct McpState {
    pub game_data: Arc<RwLock<GameData>>,
    pub session: Arc<RwLock<SessionState>>,
    sessions: Sessions,
}

impl McpState {
    pub fn new(game_data: Arc<RwLock<GameData>>, session: Arc<RwLock<SessionState>>) -> Self {
        Self {
            game_data,
            session,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub async fn start(mcp_state: McpState, port: u16) {
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/messages", post(messages_handler))
        .with_state(mcp_state);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind MCP local port");

    tracing::info!("MCP SSE Server listening on {addr}");
    axum::serve(listener, app)
        .await
        .expect("Failed to serve MCP SSE Server");
}

async fn sse_handler(State(state): State<McpState>) -> impl IntoResponse {
    let session_id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst).to_string();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    state.sessions.lock().await.insert(session_id.clone(), tx);

    let endpoint_url = format!("/messages?sessionId={session_id}");
    let endpoint_stream = tokio_stream::iter(vec![Ok::<Event, Infallible>(
        Event::default().event("endpoint").data(endpoint_url),
    )]);
    let data_stream = UnboundedReceiverStream::new(rx)
        .map(|msg| Ok::<Event, Infallible>(Event::default().data(msg)));

    Sse::new(endpoint_stream.chain(data_stream)).keep_alive(KeepAlive::default())
}

#[derive(Deserialize)]
struct SessionQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

async fn messages_handler(
    State(state): State<McpState>,
    Query(query): Query<SessionQuery>,
    axum::extract::Json(body): axum::extract::Json<Value>,
) -> impl IntoResponse {
    let session_id = match query.session_id {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let tx = state.sessions.lock().await.get(&session_id).cloned();
    let tx = match tx {
        Some(tx) => tx,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if let Some(response) = handle_jsonrpc(state, body) {
        let _ = tx.send(serde_json::to_string(&response).unwrap_or_default());
    }

    StatusCode::ACCEPTED.into_response()
}

fn handle_jsonrpc(state: McpState, request: Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = request
        .get("params")
        .cloned()
        .unwrap_or(Value::Object(Map::default()));

    // Notifications have no id — no response required
    id.as_ref()?;
    let id = id.unwrap();

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "nwn2-editor", "version": "1.0.0" }
        })),
        "tools/list" => Ok(json!({ "tools": tools_list() })),
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Map::default()));
            match dispatch_tool(state, tool_name, arguments) {
                Ok(value) => Ok(json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }],
                    "isError": false
                })),
                Err(e) => Ok(json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                })),
            }
        }
        _ => Err(json!({ "code": -32601, "message": format!("Method not found: {method}") })),
    };

    Some(match result {
        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
        Err(err) => json!({ "jsonrpc": "2.0", "id": id, "error": err }),
    })
}

fn tools_list() -> Value {
    json!([
        {
            "name": "query_2da",
            "description": "Query a specific row from a NWN2 2DA game data table by row ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Name of the 2DA table (e.g. 'classes', 'feat', 'spells')" },
                    "row_id": { "type": "integer", "description": "Row ID to retrieve" }
                },
                "required": ["table", "row_id"]
            }
        },
        {
            "name": "search_2da",
            "description": "Search a NWN2 2DA table by string query (max 50 results)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Name of the 2DA table" },
                    "query": { "type": "string", "description": "Search string (case-insensitive)" }
                },
                "required": ["table", "query"]
            }
        },
        {
            "name": "get_tlk_string",
            "description": "Retrieve a TLK dialog string by its string reference ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "strref": { "type": "integer", "description": "TLK string reference ID" }
                },
                "required": ["strref"]
            }
        },
        {
            "name": "get_overview",
            "description": "Get character overview: name, race, level, ability scores, saves, BAB, HP, alignment",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_classes",
            "description": "Get full class progression with level history, feat/skill gains per level, skill point summary",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_feats",
            "description": "Get all feats with availability, prerequisites, and descriptions",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_spells",
            "description": "Get known/memorized spells and spell caster info",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_skills",
            "description": "Get all skills with ranks, modifiers, bonuses, and calculated totals",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_abilities",
            "description": "Get ability scores (base/effective), modifiers, point buy tracking",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_saves",
            "description": "Get all saving throws (fort/ref/will) with ability, base, and equipment breakdowns",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_inventory",
            "description": "Get equipped items and inventory with decoded item properties",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_campaign_info",
            "description": "Get campaign variables, quest tracking, companion status",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "query_character",
            "description": "Get basic character info: name, level, experience, class IDs",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

fn dispatch_tool(state: McpState, tool_name: &str, params: Value) -> anyhow::Result<Value> {
    match tool_name {
        "query_2da" => query_2da(state, params),
        "search_2da" => search_2da(state, params),
        "get_tlk_string" => query_tlk(state, params),
        "query_character" => query_character(state, params),
        "get_overview" => get_overview(state, params),
        "get_classes" => get_classes(state, params),
        "get_feats" => get_feats(state, params),
        "get_spells" => get_spells(state, params),
        "get_skills" => get_skills(state, params),
        "get_abilities" => get_abilities(state, params),
        "get_saves" => get_saves(state, params),
        "get_inventory" => get_inventory(state, params),
        "get_campaign_info" => get_campaign_info(state, params),
        _ => Err(anyhow::anyhow!("Unknown tool: {tool_name}")),
    }
}

// --- Tool handlers (unchanged) ---

fn query_2da(state: McpState, params: Value) -> anyhow::Result<Value> {
    let table = params
        .get("table")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing parameter: table"))?;

    let row_id = params
        .get("row_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as usize)
        .ok_or_else(|| anyhow::anyhow!("Missing parameter: row_id"))?;

    let game_data_lock = state.game_data.read();
    if game_data_lock.table_count() == 0 {
        return Err(anyhow::anyhow!(
            "Game Data not initialized. Run the editor and load a character first."
        ));
    }

    let table_ref = game_data_lock
        .get_table(table)
        .ok_or_else(|| anyhow::anyhow!("Table {table} not found"))?;

    match table_ref.get_row(row_id) {
        Ok(row) => {
            let mut tree = std::collections::BTreeMap::new();
            for (k, v) in row {
                tree.insert(k, v);
            }
            Ok(serde_json::to_value(tree)?)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to retrieve 2DA row: {e}")),
    }
}

fn search_2da(state: McpState, params: Value) -> anyhow::Result<Value> {
    let table = params
        .get("table")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing parameter: table"))?;

    let query = params
        .get("query")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("Missing parameter: query"))?;

    let game_data_lock = state.game_data.read();
    if game_data_lock.table_count() == 0 {
        return Err(anyhow::anyhow!(
            "Game Data not initialized. Run the editor and load a character first."
        ));
    }

    let table_ref = game_data_lock
        .get_table(table)
        .ok_or_else(|| anyhow::anyhow!("Table {table} not found"))?;

    let mut results = Vec::new();
    for row_id in 0..table_ref.row_count() {
        if let Ok(row) = table_ref.get_row(row_id) {
            let mut matches = false;
            for val in row.values() {
                if let Some(s) = val
                    && s.to_lowercase().contains(&query)
                {
                    matches = true;
                    break;
                }
            }
            if matches {
                let mut tree = std::collections::BTreeMap::new();
                for (k, v) in row {
                    if let Some(s) = v {
                        tree.insert(k.clone(), s.clone());
                    } else {
                        tree.insert(k.clone(), String::new());
                    }
                }
                tree.insert("id".to_string(), row_id.to_string());
                results.push(tree);
                if results.len() >= 50 {
                    break;
                }
            }
        }
    }

    Ok(serde_json::to_value(results)?)
}

fn query_tlk(state: McpState, params: Value) -> anyhow::Result<Value> {
    let strref = params
        .get("strref")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| anyhow::anyhow!("Missing parameter: strref"))?;

    let game_data_lock = state.game_data.read();
    if game_data_lock.table_count() == 0 {
        return Err(anyhow::anyhow!("Game Data not initialized."));
    }

    if let Some(text) = game_data_lock.get_string(strref) {
        Ok(json!({ "text": text }))
    } else {
        Err(anyhow::anyhow!("StrRef {strref} not found"))
    }
}

fn query_character(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();

    if let Some(char_ref) = session_lock.character() {
        Ok(json!({
            "name": char_ref.full_name(),
            "level": char_ref.total_level(),
            "experience": char_ref.experience(),
            "classes": char_ref.class_entries().iter().map(|c| c.class_id.0).collect::<Vec<_>>(),
        }))
    } else {
        Err(anyhow::anyhow!(
            "No character is currently loaded in the session. Please open a character in the editor UI first."
        ))
    }
}

fn get_overview(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        let decoder = &session_lock.item_property_decoder;
        Ok(serde_json::to_value(
            char_ref.get_overview_state(&game_data_lock, decoder),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_classes(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        Ok(serde_json::to_value(
            char_ref.get_classes_state(&game_data_lock),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_feats(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        Ok(serde_json::to_value(
            char_ref.get_feats_state(&game_data_lock),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_spells(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        Ok(serde_json::to_value(
            char_ref.get_spells_state(&game_data_lock),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_skills(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        let decoder = &session_lock.item_property_decoder;
        Ok(serde_json::to_value(
            char_ref.get_skill_summary(&game_data_lock, Some(decoder)),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_abilities(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        let decoder = &session_lock.item_property_decoder;
        Ok(serde_json::to_value(
            char_ref.get_abilities_state(&game_data_lock, decoder),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_saves(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        let decoder = &session_lock.item_property_decoder;
        Ok(serde_json::to_value(
            char_ref.get_save_summary(&game_data_lock, decoder),
        )?)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_inventory(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    let game_data_lock = state.game_data.read();
    if let Some(char_ref) = session_lock.character() {
        let decoder = &session_lock.item_property_decoder;
        let summary = char_ref.get_full_inventory_summary(&game_data_lock, decoder, None);
        let mut json_val = serde_json::to_value(summary)?;
        if let Some(obj) = json_val.as_object_mut() {
            if let Some(inv) = obj.get_mut("inventory").and_then(|v| v.as_array_mut()) {
                for item in inv {
                    if let Some(item_obj) = item.as_object_mut() {
                        item_obj.remove("item");
                    }
                }
            }
            if let Some(equipped) = obj.get_mut("equipped").and_then(|v| v.as_array_mut()) {
                for item in equipped {
                    if let Some(item_obj) = item.as_object_mut() {
                        item_obj.remove("item_data");
                    }
                }
            }
        }
        Ok(json_val)
    } else {
        Err(anyhow::anyhow!("No character is currently loaded."))
    }
}

fn get_campaign_info(state: McpState, _params: Value) -> anyhow::Result<Value> {
    let session_lock = state.session.read();
    if let Some(handler) = session_lock.savegame_handler.as_ref() {
        let summary = crate::services::campaign::CampaignManager::get_summary(handler)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(serde_json::to_value(summary)?)
    } else {
        Err(anyhow::anyhow!("No savegame is currently loaded."))
    }
}
