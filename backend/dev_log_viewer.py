"""
Development Log Viewer - ONLY runs when ENABLE_LOG_VIEWER=true
Standalone server on port 9999 with web UI for filtering logs
"""
import os
import sys
from pathlib import Path
from typing import List, Optional
from fastapi import FastAPI, Query
from fastapi.responses import HTMLResponse, JSONResponse
import uvicorn

if os.getenv("ENABLE_LOG_VIEWER", "false").lower() != "true":
    print("Log viewer is disabled. Set ENABLE_LOG_VIEWER=true to enable.")
    sys.exit(0)

LOG_DIR = Path(__file__).parent / "logs"
LOG_FILE = LOG_DIR / "app.log"

app = FastAPI(title="NWN2 Editor Dev Log Viewer")

def get_available_sessions() -> List[str]:
    """Extract unique session IDs from logs"""
    if not LOG_FILE.exists():
        return []

    sessions = set()
    try:
        with open(LOG_FILE, "r", encoding="utf-8") as f:
            for line in f:
                if " | [" in line and "] | " in line:
                    # Extract session ID between [ and ]
                    start = line.find("[") + 1
                    end = line.find("]", start)
                    if end > start:
                        session_id = line[start:end].strip()
                        sessions.add(session_id)
    except Exception:
        pass

    return sorted(list(sessions), reverse=True)  # Newest first

def get_available_modules() -> List[str]:
    """Extract unique module names from logs"""
    if not LOG_FILE.exists():
        return []

    modules = set()
    try:
        with open(LOG_FILE, "r", encoding="utf-8") as f:
            for line in f:
                if " | " in line:
                    parts = line.split(" | ")
                    if len(parts) >= 4:  # Now has session ID
                        module_part = parts[3].split(":")[0].strip()
                        # Skip __main__ and generic modules, keep application modules
                        if module_part and module_part != "__main__" and "." in module_part:
                            modules.add(module_part)
    except Exception:
        pass

    return sorted(list(modules))

def read_logs(
    module_filter: Optional[str] = None,
    level_filter: Optional[str] = None,
    search: Optional[str] = None,
    session_filter: Optional[str] = None,
    tail: int = 500
) -> List[str]:
    """Read and filter logs"""
    if not LOG_FILE.exists():
        return ["No logs available yet"]

    try:
        with open(LOG_FILE, "r", encoding="utf-8") as f:
            lines = f.readlines()

        lines = lines[-tail:]

        filtered = []
        for line in lines:
            if session_filter and f"[{session_filter}]" not in line:
                continue
            if module_filter and module_filter not in line:
                continue
            if level_filter and f"| {level_filter: <8} |" not in line:
                continue
            if search and search.lower() not in line.lower():
                continue
            filtered.append(line.rstrip())

        return filtered if filtered else ["No logs match your filters"]
    except Exception as e:
        return [f"Error reading logs: {str(e)}"]

@app.get("/api/sessions")
async def get_sessions():
    """API endpoint for fetching available sessions"""
    sessions = get_available_sessions()
    return JSONResponse({"sessions": sessions})

@app.get("/api/modules")
async def get_modules():
    """API endpoint for fetching available modules"""
    modules = get_available_modules()
    return JSONResponse({"modules": modules})

@app.get("/", response_class=HTMLResponse)
async def log_viewer():
    """Main log viewer UI"""
    html = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <title>NWN2 Editor Dev Logs</title>
        <style>
            body {{
                font-family: 'Consolas', 'Monaco', monospace;
                background: #1e1e1e;
                color: #d4d4d4;
                margin: 0;
                padding: 20px;
            }}
            .controls {{
                background: #252526;
                padding: 15px;
                border-radius: 5px;
                margin-bottom: 20px;
                display: flex;
                gap: 15px;
                flex-wrap: wrap;
                align-items: center;
            }}
            select, input, button {{
                padding: 8px 12px;
                background: #3c3c3c;
                color: #d4d4d4;
                border: 1px solid #555;
                border-radius: 3px;
                font-family: inherit;
                font-size: 14px;
            }}
            button {{
                background: #0e639c;
                cursor: pointer;
            }}
            button:hover {{
                background: #1177bb;
            }}
            #logs {{
                background: #1e1e1e;
                border: 1px solid #3c3c3c;
                border-radius: 5px;
                padding: 15px;
                overflow-x: auto;
                white-space: pre;
                font-size: 13px;
                line-height: 1.6;
                max-height: calc(100vh - 200px);
                overflow-y: auto;
            }}
            .log-line {{
                margin: 2px 0;
            }}
            .DEBUG {{ color: #858585; }}
            .INFO {{ color: #4ec9b0; }}
            .WARNING {{ color: #dcdcaa; }}
            .ERROR {{ color: #f48771; }}
            .CRITICAL {{ color: #f14c4c; }}
            label {{
                font-size: 14px;
            }}
            .status {{
                margin-left: auto;
                color: #858585;
                font-size: 12px;
            }}
        </style>
    </head>
    <body>
        <h1>🔍 NWN2 Editor Dev Logs</h1>

        <div class="controls">
            <div>
                <label>Session:</label>
                <select id="sessionFilter">
                    <option value="">All Sessions</option>
                </select>
            </div>

            <div>
                <label>Module:</label>
                <select id="moduleFilter">
                    <option value="">All Modules</option>
                </select>
            </div>

            <div>
                <label>Level:</label>
                <select id="levelFilter">
                    <option value="">All Levels</option>
                    <option value="DEBUG">DEBUG</option>
                    <option value="INFO">INFO</option>
                    <option value="WARNING">WARNING</option>
                    <option value="ERROR">ERROR</option>
                    <option value="CRITICAL">CRITICAL</option>
                </select>
            </div>

            <div>
                <label>Search:</label>
                <input type="text" id="searchInput" placeholder="Search logs...">
            </div>

            <button onclick="loadLogs()">Refresh</button>
            <button onclick="clearFilters()">Clear Filters</button>
            <button onclick="toggleAutoRefresh()">Auto-Refresh: <span id="autoStatus">OFF</span></button>

            <div class="status" id="status">Ready</div>
        </div>

        <div id="logs">Loading logs...</div>

        <script>
            let autoRefresh = false;
            let autoRefreshInterval = null;

            async function loadSessions() {{
                try {{
                    const response = await fetch('./api/sessions');
                    const data = await response.json();

                    const select = document.getElementById('sessionFilter');
                    select.innerHTML = '<option value="">All Sessions</option>';

                    data.sessions.forEach((session, index) => {{
                        const option = document.createElement('option');
                        option.value = session;
                        option.textContent = index === 0 ? `${{session}} (current)` : session;
                        if (index === 0) option.selected = true;  // Auto-select current session
                        select.appendChild(option);
                    }});
                }} catch (error) {{
                    console.error('Failed to load sessions:', error);
                }}
            }}

            async function loadModules() {{
                try {{
                    const response = await fetch('./api/modules');
                    const data = await response.json();

                    const select = document.getElementById('moduleFilter');
                    select.innerHTML = '<option value="">All Modules</option>';

                    data.modules.forEach(module => {{
                        const option = document.createElement('option');
                        option.value = module;
                        option.textContent = module;
                        select.appendChild(option);
                    }});
                }} catch (error) {{
                    console.error('Failed to load modules:', error);
                }}
            }}

            function highlightLog(line) {{
                const levels = ['DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'];
                for (const level of levels) {{
                    if (line.includes(`| ${{level.padEnd(8)}} |`)) {{
                        return `<div class="log-line ${{level}}">${{escapeHtml(line)}}</div>`;
                    }}
                }}
                return `<div class="log-line">${{escapeHtml(line)}}</div>`;
            }}

            function escapeHtml(text) {{
                const div = document.createElement('div');
                div.textContent = text;
                return div.innerHTML;
            }}

            async function loadLogs() {{
                const session = document.getElementById('sessionFilter').value;
                const module = document.getElementById('moduleFilter').value;
                const level = document.getElementById('levelFilter').value;
                const search = document.getElementById('searchInput').value;

                document.getElementById('status').textContent = 'Loading...';

                const params = new URLSearchParams();
                if (session) params.append('session', session);
                if (module) params.append('module', module);
                if (level) params.append('level', level);
                if (search) params.append('search', search);

                try {{
                    const response = await fetch(`./api/logs?${{params}}`);
                    const data = await response.json();

                    const logsDiv = document.getElementById('logs');
                    logsDiv.innerHTML = data.logs.map(highlightLog).join('');
                    logsDiv.scrollTop = logsDiv.scrollHeight;

                    document.getElementById('status').textContent = `Loaded ${{data.count}} logs`;
                }} catch (error) {{
                    document.getElementById('logs').innerHTML = `<div class="ERROR">Error loading logs: ${{error}}</div>`;
                    document.getElementById('status').textContent = 'Error';
                }}
            }}

            function clearFilters() {{
                document.getElementById('sessionFilter').value = '';
                document.getElementById('moduleFilter').value = '';
                document.getElementById('levelFilter').value = '';
                document.getElementById('searchInput').value = '';
                loadLogs();
            }}

            function toggleAutoRefresh() {{
                autoRefresh = !autoRefresh;
                const status = document.getElementById('autoStatus');
                status.textContent = autoRefresh ? 'ON' : 'OFF';

                if (autoRefresh) {{
                    loadLogs();
                    autoRefreshInterval = setInterval(loadLogs, 2000);
                }} else {{
                    if (autoRefreshInterval) {{
                        clearInterval(autoRefreshInterval);
                        autoRefreshInterval = null;
                    }}
                }}
            }}

            document.getElementById('sessionFilter').addEventListener('change', loadLogs);
            document.getElementById('moduleFilter').addEventListener('change', loadLogs);
            document.getElementById('levelFilter').addEventListener('change', loadLogs);
            document.getElementById('searchInput').addEventListener('input', loadLogs);

            // Load sessions, modules and logs on page load (in sequence)
            async function initializePage() {{
                await loadSessions();  // Load sessions first
                await loadModules();   // Then modules
                loadLogs();            // Then logs with current session selected
            }}

            initializePage();
        </script>
    </body>
    </html>
    """
    return html

@app.get("/api/logs")
async def get_logs(
    module: Optional[str] = Query(None),
    level: Optional[str] = Query(None),
    search: Optional[str] = Query(None),
    session: Optional[str] = Query(None),
    tail: int = Query(500)
):
    """API endpoint for fetching filtered logs"""
    logs = read_logs(module, level, search, session, tail)
    return JSONResponse({
        "logs": logs,
        "count": len(logs)
    })

if __name__ == "__main__":
    print(f"🔍 Starting Dev Log Viewer on http://localhost:9999")
    print(f"   Log file: {LOG_FILE}")
    print(f"   Press Ctrl+C to stop")
    uvicorn.run(app, host="127.0.0.1", port=9999, log_level="warning")
