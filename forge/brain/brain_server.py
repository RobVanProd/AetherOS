#!/usr/bin/env python3
"""
AetherOS Brain Server — The intelligence core.

Receives natural language input, executes relevant tools locally,
then calls Claude via CLI for natural language response generation.

Usage:
    python3 brain_server.py --port 9200
"""

import json
import os
import re
import subprocess
import sys
import time
import traceback
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import quote as url_quote

try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

DEFAULT_PORT = 9200
CLAUDE_MODEL = "sonnet"
MAX_HISTORY = 20
HOME = os.path.expanduser("~")

# Ollama config for local model proactive insights
OLLAMA_URL = os.environ.get("OLLAMA_URL", "http://localhost:11434")
OLLAMA_MODEL = os.environ.get("OLLAMA_MODEL", "phi3:mini")
USE_LOCAL_MODEL = os.environ.get("USE_LOCAL_MODEL", "false") == "true"

BRAIN_SYSTEM_PROMPT = """You are the brain of AetherOS, a generative AI-native operating system built by Aeternum Labs. You run inside the OS. The user types natural language into the omni-bar and you respond with exactly what they need.

You are NOT a chatbot — you ARE the operating system's intelligence.

Your responses are rendered in an 80-column terminal. Keep text concise and well-formatted.

IMPORTANT: You MUST respond with ONLY valid JSON in this exact format — no markdown fences, no extra text:
{"text": "Your human-readable response here", "widgets": []}

Widget types you can include in the widgets array:
- {"type": "weather", "title": "NYC Weather", "lines": ["45°F  Cloudy", "Humidity: 72%  Wind: 12mph NW", "Today: 48/38 Cloudy", "Tomorrow: 52/41 Partly Sunny"]}
- {"type": "table", "title": "Files", "lines": ["filename.txt     4.2KB    2026-02-01", "notes.md         1.8KB    2026-02-03"]}
- {"type": "file", "title": "~/docs/notes.txt", "lines": ["Line 1 of file content", "Line 2 of file content"]}
- {"type": "system", "title": "System Info", "lines": ["CPU: 12.3% (4 cores)", "Memory: 128/489MB", "Uptime: 5m 23s"]}
- {"type": "info", "title": "Title", "lines": ["Line 1", "Line 2"]}

When no widgets are needed (creative writing, simple answers), use an empty widgets array.
Be direct, helpful, and concise. You are the OS — act like it. No emoji unless asked."""

# ---------------------------------------------------------------------------
# Tool implementations (executed locally before calling Claude)
# ---------------------------------------------------------------------------

def tool_weather(location: str) -> dict:
    """Fetch weather data from wttr.in."""
    try:
        if HAS_REQUESTS:
            resp = requests.get(
                f"https://wttr.in/{url_quote(location)}?format=j1",
                timeout=10,
                headers={"User-Agent": "AetherOS-Brain/0.3"}
            )
            if resp.status_code == 200:
                return {"ok": True, "data": resp.json()}
            return {"ok": False, "error": f"HTTP {resp.status_code}"}
        else:
            result = subprocess.run(
                ["curl", "-sf", f"https://wttr.in/{url_quote(location)}?format=j1",
                 "-H", "User-Agent: AetherOS-Brain/0.3"],
                capture_output=True, text=True, timeout=10
            )
            if result.returncode == 0:
                return {"ok": True, "data": json.loads(result.stdout)}
            return {"ok": False, "error": "fetch failed"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_read_file(path: str, max_lines: int = 100) -> dict:
    try:
        p = Path(path).expanduser().resolve()
        if not p.exists():
            return {"ok": False, "error": f"File not found: {path}"}
        text = p.read_text(errors="replace")
        lines = text.splitlines()
        truncated = len(lines) > max_lines
        if truncated:
            lines = lines[:max_lines]
        return {"ok": True, "path": str(p), "lines": lines, "total_lines": len(text.splitlines()), "truncated": truncated}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_write_file(path: str, content: str) -> dict:
    try:
        p = Path(path).expanduser().resolve()
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content)
        return {"ok": True, "path": str(p), "bytes": len(content)}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_search_files(query: str, path: str = "~", by_name: bool = False) -> dict:
    try:
        search_dir = str(Path(path).expanduser().resolve())
        if by_name:
            result = subprocess.run(
                ["find", search_dir, "-maxdepth", "4", "-name", f"*{query}*", "-type", "f"],
                capture_output=True, text=True, timeout=10
            )
        else:
            result = subprocess.run(
                ["grep", "-rl", "-m", "1", query, search_dir,
                 "--include=*.txt", "--include=*.md", "--include=*.py",
                 "--include=*.rs", "--include=*.toml", "--include=*.json",
                 "--include=*.yaml", "--include=*.yml", "--include=*.sh"],
                capture_output=True, text=True, timeout=10
            )
        files = [f for f in result.stdout.strip().split("\n") if f][:20]
        return {"ok": True, "files": files, "count": len(files)}
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": "Search timed out", "files": []}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_list_files(path: str = "~") -> dict:
    try:
        p = Path(path).expanduser().resolve()
        if not p.is_dir():
            return {"ok": False, "error": f"Not a directory: {path}"}
        entries = []
        for entry in sorted(p.iterdir()):
            try:
                stat = entry.stat()
                entries.append({
                    "name": entry.name + ("/" if entry.is_dir() else ""),
                    "size": stat.st_size,
                    "modified": time.strftime("%Y-%m-%d %H:%M", time.localtime(stat.st_mtime))
                })
            except (PermissionError, OSError):
                continue
        return {"ok": True, "path": str(p), "entries": entries[:50]}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_system_info() -> dict:
    info = {}
    try:
        with open("/proc/uptime") as f:
            secs = int(float(f.read().split()[0]))
            mins, s = divmod(secs, 60)
            hours, mins = divmod(mins, 60)
            info["uptime"] = f"{hours}h {mins}m {s}s" if hours else f"{mins}m {s}s"
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    info["mem_total_mb"] = int(line.split()[1]) // 1024
                elif line.startswith("MemAvailable:"):
                    info["mem_avail_mb"] = int(line.split()[1]) // 1024
        with open("/proc/cpuinfo") as f:
            info["cores"] = sum(1 for line in f if line.startswith("processor"))
        with open("/proc/loadavg") as f:
            info["load"] = f.read().split()[:3]
        result = subprocess.run(["df", "-h", "/"], capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            parts = result.stdout.strip().split("\n")[-1].split()
            info["disk"] = f"{parts[2]}/{parts[1]} ({parts[4]})"
        result = subprocess.run(["hostname", "-I"], capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            info["ip"] = result.stdout.strip().split()[0] if result.stdout.strip() else "N/A"
    except Exception as e:
        info["error"] = str(e)
    return {"ok": True, **info}


def tool_web_fetch(url: str) -> dict:
    try:
        if HAS_REQUESTS:
            resp = requests.get(url, timeout=10, headers={"User-Agent": "AetherOS-Brain/0.3"})
            text = resp.text[:4000]
        else:
            result = subprocess.run(
                ["curl", "-sf", "-L", url, "-H", "User-Agent: AetherOS-Brain/0.3"],
                capture_output=True, text=True, timeout=10
            )
            text = result.stdout[:4000]
        # Strip HTML tags for readability
        text = re.sub(r'<[^>]+>', ' ', text)
        text = re.sub(r'\s+', ' ', text).strip()
        return {"ok": True, "content": text}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def tool_run_command(command: str) -> dict:
    try:
        result = subprocess.run(
            command, shell=True, capture_output=True, text=True, timeout=15
        )
        output = result.stdout
        if result.stderr:
            output += "\n" + result.stderr
        return {"ok": True, "output": output[:4000] if output else "(no output)", "exit_code": result.returncode}
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": "Command timed out after 15s"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


# ---------------------------------------------------------------------------
# Intent detection — decides which tools to run BEFORE calling Claude
# ---------------------------------------------------------------------------

def detect_and_run_tools(user_input: str) -> str:
    """Analyze user input, run relevant tools, return context string for Claude."""
    lower = user_input.lower()
    context_parts = []

    # Weather detection
    weather_match = re.search(r'weather\s+(?:in\s+|for\s+|at\s+)?(.+?)(?:\?|$|\.)', lower)
    if not weather_match and 'weather' in lower:
        # Try broader match
        words = lower.split()
        if 'weather' in words:
            idx = words.index('weather')
            location = ' '.join(words[idx+1:]).strip(' ?.')
            if not location:
                location = 'here'
            weather_match = True
        else:
            weather_match = None
            location = None
    else:
        location = weather_match.group(1).strip(' ?.') if weather_match else None

    if weather_match and location and location != 'here':
        result = tool_weather(location)
        if result["ok"]:
            data = result["data"]
            current = data.get("current_condition", [{}])[0]
            context_parts.append(f"[WEATHER DATA for {location}]\n{json.dumps(current, indent=2)}")
            forecasts = data.get("weather", [])[:3]
            if forecasts:
                context_parts.append(f"[FORECAST]\n{json.dumps(forecasts, indent=2)}")
        else:
            context_parts.append(f"[WEATHER ERROR] {result['error']}")

    # File reading — match paths (case-insensitive verb, case-preserving path)
    read_match = re.search(r'(?:read|open|show|cat|view|display|look at)\s+(?:me\s+)?(?:the\s+)?(?:file\s+|contents?\s+of\s+)?["\']?([~/][\w./-]+)["\']?', user_input, re.IGNORECASE)
    if read_match:
        filepath = read_match.group(1)
        result = tool_read_file(filepath)
        if result["ok"]:
            context_parts.append(f"[FILE: {result['path']}]\n" + "\n".join(result["lines"][:50]))
        else:
            context_parts.append(f"[FILE ERROR] {result['error']}")

    # File search — flexible patterns
    search_patterns = [
        r'(?:find|search|look for|where is|locate)\s+(?:files?\s+)?(?:about\s+|for\s+|named\s+|called\s+|containing\s+|with\s+|related to\s+)?["\']?(.+?)["\']?(?:\?|$|\.)',
        r'(?:what|which)\s+files?\s+(?:do\s+I\s+have\s+)?(?:about|for|on|related to|containing|with)\s+["\']?(.+?)["\']?(?:\?|$|\.)',
        r'files?\s+(?:about|for|on|related to|containing|with)\s+["\']?(.+?)["\']?(?:\?|$|\.)',
    ]
    search_match = None
    for pat in search_patterns:
        search_match = re.search(pat, lower)
        if search_match:
            break
    if search_match:
        query = search_match.group(1).strip()
        # Remove trailing filler words
        query = re.sub(r'\s+(language|project|file|code|stuff)$', '', query)
        if query and len(query) > 1:
            result = tool_search_files(query, "~", by_name=True)
            if result["ok"] and result["files"]:
                context_parts.append(f"[SEARCH RESULTS for '{query}']\n" + "\n".join(result["files"]))
            # Also try content search
            result2 = tool_search_files(query)
            if result2["ok"] and result2["files"]:
                context_parts.append(f"[CONTENT SEARCH for '{query}']\n" + "\n".join(result2["files"]))

    # List files
    ls_match = re.search(r'(?:list|ls|what(?:\'s| is) in)\s+(?:files?\s+(?:in\s+)?)?["\']?([~/\w./-]+)["\']?', lower)
    if ls_match:
        dirpath = ls_match.group(1)
        result = tool_list_files(dirpath)
        if result["ok"]:
            entries = result["entries"][:30]
            lines = [f"{e['name']:30s} {e['size']:>8d}  {e['modified']}" for e in entries]
            context_parts.append(f"[DIRECTORY: {result['path']}]\n" + "\n".join(lines))

    # System info
    if any(kw in lower for kw in ['system info', 'sysinfo', 'uptime', 'how long', 'cpu', 'memory usage', 'disk space', 'system status']):
        result = tool_system_info()
        context_parts.append(f"[SYSTEM INFO]\n{json.dumps(result, indent=2)}")

    # Web fetch
    url_match = re.search(r'(?:fetch|get|visit|open)\s+(https?://\S+)', lower)
    if url_match:
        result = tool_web_fetch(url_match.group(1))
        if result["ok"]:
            context_parts.append(f"[WEB CONTENT]\n{result['content'][:2000]}")

    # Command execution (explicit ! prefix handled at a higher level)
    cmd_match = re.search(r'(?:run|execute)\s+(?:command\s+)?["`](.+?)["`]', lower)
    if cmd_match:
        result = tool_run_command(cmd_match.group(1))
        if result["ok"]:
            context_parts.append(f"[COMMAND OUTPUT]\n{result['output']}")

    if context_parts:
        return "\n\n".join(context_parts)
    return ""


# ---------------------------------------------------------------------------
# Claude CLI integration
# ---------------------------------------------------------------------------

def call_claude(user_input: str, tool_context: str, history: list) -> str:
    """Call Claude via CLI with tool context and conversation history."""
    # Build the prompt with context
    parts = []

    # Include recent conversation history (last 6 exchanges)
    recent = history[-(MAX_HISTORY * 2):] if len(history) > MAX_HISTORY * 2 else history
    if recent:
        parts.append("Recent conversation:")
        for msg in recent[-6:]:
            role = msg["role"].upper()
            content = msg["content"][:200]
            parts.append(f"  {role}: {content}")
        parts.append("")

    if tool_context:
        parts.append("Context data (use this to form your response):")
        parts.append(tool_context)
        parts.append("")

    parts.append(f"User query: {user_input}")
    parts.append("")
    parts.append("Respond with ONLY valid JSON: {\"text\": \"...\", \"widgets\": [...]}")

    full_prompt = "\n".join(parts)

    # Call claude CLI from /tmp to avoid loading project context
    try:
        result = subprocess.run(
            ["claude", "-p", "--model", CLAUDE_MODEL,
             "--system-prompt", BRAIN_SYSTEM_PROMPT,
             "--output-format", "json",
             "--no-session-persistence",
             "--max-budget-usd", "0.50",
             full_prompt],
            capture_output=True, text=True, timeout=90,
            cwd="/tmp",
            env={**os.environ,
                 "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
                 "DISABLE_AUTOUPDATER": "1"}
        )
        if result.returncode != 0:
            stderr = result.stderr[:200]
            return json.dumps({"text": f"Brain error: {stderr}", "widgets": []})

        # Parse the JSON output from claude CLI
        output = result.stdout.strip()
        cli_result = json.loads(output)
        response_text = cli_result.get("result", "")

        # Strip markdown code fences if present
        cleaned = response_text.strip()
        if cleaned.startswith("```"):
            # Remove ```json ... ``` wrapper
            cleaned = re.sub(r'^```(?:json)?\s*\n?', '', cleaned)
            cleaned = re.sub(r'\n?```\s*$', '', cleaned)
            cleaned = cleaned.strip()

        # Try to parse Claude's response as our JSON format
        for attempt in [cleaned, response_text]:
            try:
                parsed = json.loads(attempt)
                if "text" in parsed:
                    return json.dumps(parsed)
            except (json.JSONDecodeError, KeyError):
                pass

        # Try to extract JSON object containing "text" key
        json_match = re.search(r'\{.*"text"\s*:.*\}', cleaned, re.DOTALL)
        if json_match:
            try:
                parsed = json.loads(json_match.group())
                if "text" in parsed:
                    return json.dumps(parsed)
            except json.JSONDecodeError:
                pass

        # If still not JSON, wrap it
        return json.dumps({"text": response_text, "widgets": []})

    except subprocess.TimeoutExpired:
        return json.dumps({"text": "Brain timed out. Try a simpler query.", "widgets": []})
    except Exception as e:
        return json.dumps({"text": f"Brain error: {e}", "widgets": []})


# ---------------------------------------------------------------------------
# Brain logic
# ---------------------------------------------------------------------------

BRAIN_PROACTIVE_PROMPT = """You are AetherOS's proactive intelligence. Given system context, decide if there's anything worth telling the user.

Rules:
- Only respond if there's genuinely useful or interesting information
- Be concise: 1-2 sentences max
- Suggestions should be actionable
- Don't repeat what's obvious from the dashboard numbers
- If nothing interesting, respond {"has_insight": false}
- If interesting, respond {"has_insight": true, "text": "...", "widgets": [], "priority": "normal", "category": "observation"}
- priority: "urgent", "normal", or "low"
- category: "suggestion", "observation", or "warning"

Respond with ONLY valid JSON. No markdown fences."""


def call_ollama(prompt: str, system_prompt: str = "", timeout: int = 15) -> str:
    """Call a local Ollama model for fast proactive insights."""
    try:
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": prompt,
            "system": system_prompt,
            "stream": False,
            "options": {"temperature": 0.7, "num_predict": 200}
        }
        if HAS_REQUESTS:
            resp = requests.post(
                f"{OLLAMA_URL}/api/generate",
                json=payload, timeout=timeout
            )
            if resp.status_code == 200:
                return resp.json().get("response", "")
            return ""
        else:
            result = subprocess.run(
                ["curl", "-sf", "-X", "POST",
                 f"{OLLAMA_URL}/api/generate",
                 "-H", "Content-Type: application/json",
                 "-d", json.dumps(payload)],
                capture_output=True, text=True, timeout=timeout
            )
            if result.returncode == 0:
                return json.loads(result.stdout).get("response", "")
            return ""
    except Exception as e:
        print(f"[brain] Ollama error: {e}")
        return ""


BRAIN_DASHBOARD_PROMPT = """You are AetherOS's dashboard intelligence. Given a user's name, interests, system telemetry, and time of day, generate a personalized dashboard layout.

Respond with ONLY valid JSON (no markdown fences) in this format:
{
  "greeting": "Good afternoon, Rob.",
  "subtitle": "Here's what I found for you today.",
  "cards": [
    {"type": "system", "title": "System Health", "metrics": {"cpu": 42, "mem": 19}},
    {"type": "weather", "title": "Weather", "temp": "45F", "desc": "Cloudy", "wind": "12mph NW"},
    {"type": "text", "title": "Insight", "body": "Your system has been running smoothly for 2 hours."},
    {"type": "news", "title": "Tech News", "body": "Latest headline relevant to user interests."}
  ]
}

Card types: system, weather, text, news, tip, alert. Generate 3-5 cards based on user interests and context. Always include a system health card. Be creative and relevant."""


class Brain:
    def __init__(self):
        self.history = []
        self.last_proactive_time = 0
        self.proactive_cooldown = 60  # minimum seconds between proactive calls

    def proactive(self, context: dict) -> dict:
        """Process system context and return proactive insight if warranted."""
        now = time.time()
        if now - self.last_proactive_time < self.proactive_cooldown:
            return {"has_insight": False, "reason": "cooldown"}
        self.last_proactive_time = now

        # Build a compact context string
        parts = []
        if "telemetry" in context:
            t = context["telemetry"]
            parts.append(f"System: CPU {t.get('cpu', 0):.0f}%, Mem {t.get('mem_pct', 0):.0f}%, "
                         f"Uptime {t.get('uptime', 'unknown')}, Procs {t.get('procs', 0)}, "
                         f"Net {t.get('network', 'unknown')}")

        if "world_model" in context:
            wm = context["world_model"]
            parts.append(f"World Model: error={wm.get('prediction_error', 'N/A')}, "
                         f"trend={wm.get('trend', 'unknown')}, "
                         f"learning={wm.get('learning_enabled', False)}")

        if "recent_alerts" in context and context["recent_alerts"]:
            alerts = context["recent_alerts"][:5]
            parts.append(f"Recent alerts: {', '.join(alerts)}")

        if "user_activity" in context:
            ua = context["user_activity"]
            parts.append(f"User: last query='{ua.get('last_query', 'none')}', "
                         f"session={ua.get('session_duration', 'unknown')}")

        if "tasks" in context:
            tk = context["tasks"]
            parts.append(f"Tasks: {tk.get('active', 0)} active, {tk.get('completed', 0)} done")

        if "user_context" in context:
            uc = context["user_context"]
            if "topics" in uc:
                parts.append(f"User interests: {', '.join(uc['topics'][:5])}")

        context_str = "\n".join(parts)

        print(f"[brain] Proactive check with {len(context_str)} chars of context")

        # Route: proactive calls → local model (fast), complex queries → Claude (quality)
        if USE_LOCAL_MODEL:
            print(f"[brain] Proactive via local model ({OLLAMA_MODEL})")
            response_text = call_ollama(
                f"Current system state:\n{context_str}\n\nIs there anything the user should know?",
                system_prompt=BRAIN_PROACTIVE_PROMPT,
                timeout=10
            )
            if response_text:
                try:
                    cleaned = response_text.strip()
                    if cleaned.startswith("```"):
                        cleaned = re.sub(r'^```(?:json)?\s*\n?', '', cleaned)
                        cleaned = re.sub(r'\n?```\s*$', '', cleaned)
                    parsed = json.loads(cleaned)
                    return parsed
                except (json.JSONDecodeError, KeyError):
                    print(f"[brain] Local model parse failed, falling back to Claude")

        # Call Claude with proactive prompt (cheaper, faster)
        try:
            result = subprocess.run(
                ["claude", "-p", "--model", CLAUDE_MODEL,
                 "--system-prompt", BRAIN_PROACTIVE_PROMPT,
                 "--output-format", "json",
                 "--no-session-persistence",
                 "--max-budget-usd", "0.05",
                 f"Current system state:\n{context_str}\n\nIs there anything the user should know?"],
                capture_output=True, text=True, timeout=30,
                cwd="/tmp",
                env={**os.environ,
                     "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
                     "DISABLE_AUTOUPDATER": "1"}
            )
            if result.returncode != 0:
                return {"has_insight": False, "error": result.stderr[:100]}

            output = result.stdout.strip()
            cli_result = json.loads(output)
            response_text = cli_result.get("result", "")

            # Clean markdown fences
            cleaned = response_text.strip()
            if cleaned.startswith("```"):
                cleaned = re.sub(r'^```(?:json)?\s*\n?', '', cleaned)
                cleaned = re.sub(r'\n?```\s*$', '', cleaned)
                cleaned = cleaned.strip()

            parsed = json.loads(cleaned)
            return parsed

        except subprocess.TimeoutExpired:
            return {"has_insight": False, "error": "timeout"}
        except (json.JSONDecodeError, KeyError):
            return {"has_insight": False, "error": "parse_error"}
        except Exception as e:
            return {"has_insight": False, "error": str(e)}

    def dashboard(self, context: dict) -> dict:
        """Generate a personalized dashboard layout based on user context."""
        name = context.get("name", "User")
        interests = context.get("interests", [])
        telemetry = context.get("telemetry", {})

        # Determine time of day greeting
        hour = time.localtime().tm_hour
        if hour < 12:
            tod = "morning"
        elif hour < 17:
            tod = "afternoon"
        else:
            tod = "evening"

        prompt_parts = [
            f"User: {name}",
            f"Time: {tod}",
            f"Interests: {', '.join(interests) if interests else 'general'}",
        ]
        if telemetry:
            prompt_parts.append(f"System: CPU {telemetry.get('cpu', 0):.0f}%, Mem {telemetry.get('mem_pct', 0):.0f}%, Uptime {telemetry.get('uptime', 'unknown')}")

        context_str = "\n".join(prompt_parts)

        # Route: proactive/dashboard generation → local model if available, else Claude
        if USE_LOCAL_MODEL:
            print(f"[brain] Dashboard via local model ({OLLAMA_MODEL})")
            response_text = call_ollama(
                f"Generate a dashboard layout:\n{context_str}",
                system_prompt=BRAIN_DASHBOARD_PROMPT,
                timeout=15
            )
        else:
            response_text = ""

        # Fall back to Claude if local model didn't produce valid JSON
        parsed = None
        if response_text:
            try:
                cleaned = response_text.strip()
                if cleaned.startswith("```"):
                    cleaned = re.sub(r'^```(?:json)?\s*\n?', '', cleaned)
                    cleaned = re.sub(r'\n?```\s*$', '', cleaned)
                parsed = json.loads(cleaned)
                if "greeting" in parsed and "cards" in parsed:
                    print(f"[brain] Dashboard from local model OK")
                    return parsed
            except (json.JSONDecodeError, KeyError):
                pass

        # Use Claude for dashboard
        print(f"[brain] Dashboard via Claude ({CLAUDE_MODEL})")
        try:
            result = subprocess.run(
                ["claude", "-p", "--model", CLAUDE_MODEL,
                 "--system-prompt", BRAIN_DASHBOARD_PROMPT,
                 "--output-format", "json",
                 "--no-session-persistence",
                 "--max-budget-usd", "0.10",
                 f"Generate dashboard:\n{context_str}"],
                capture_output=True, text=True, timeout=30,
                cwd="/tmp",
                env={**os.environ,
                     "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
                     "DISABLE_AUTOUPDATER": "1"}
            )
            if result.returncode == 0:
                output = result.stdout.strip()
                cli_result = json.loads(output)
                response_text = cli_result.get("result", "")
                cleaned = response_text.strip()
                if cleaned.startswith("```"):
                    cleaned = re.sub(r'^```(?:json)?\s*\n?', '', cleaned)
                    cleaned = re.sub(r'\n?```\s*$', '', cleaned)
                parsed = json.loads(cleaned)
                if "greeting" in parsed:
                    return parsed
        except Exception as e:
            print(f"[brain] Dashboard Claude error: {e}")

        # Fallback: static dashboard
        return {
            "greeting": f"Good {tod}, {name}.",
            "subtitle": "Here's your system overview.",
            "cards": [
                {"type": "system", "title": "System Health", "metrics": {"cpu": int(telemetry.get("cpu", 0)), "mem": int(telemetry.get("mem_pct", 0))}},
                {"type": "text", "title": "Welcome", "body": "AetherOS is running. Ask me anything in the omnibar below."},
            ]
        }

    def query(self, user_input: str) -> dict:
        """Process a natural language query and return structured response."""
        # Step 1: Detect intent and run tools locally
        print(f"[brain] Detecting intent for: {user_input[:60]}")
        tool_context = detect_and_run_tools(user_input)
        if tool_context:
            print(f"[brain] Tool context: {len(tool_context)} chars")

        # Step 2: Call Claude with context
        print(f"[brain] Calling Claude ({CLAUDE_MODEL})...")
        start = time.time()
        response_json = call_claude(user_input, tool_context, self.history)
        elapsed = time.time() - start
        print(f"[brain] Claude responded in {elapsed:.1f}s")

        # Step 3: Parse and return
        try:
            result = json.loads(response_json)
        except json.JSONDecodeError:
            result = {"text": response_json, "widgets": []}

        # Update history
        self.history.append({"role": "user", "content": user_input})
        self.history.append({"role": "assistant", "content": result.get("text", "")})

        # Trim history
        if len(self.history) > MAX_HISTORY * 2:
            self.history = self.history[-(MAX_HISTORY * 2):]

        return result


# ---------------------------------------------------------------------------
# HTTP server
# ---------------------------------------------------------------------------

brain_instance = None

class BrainHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        print(f"[brain] {args[0]}")

    def _send_json(self, status: int, data: dict):
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/v0/health":
            self._send_json(200, {"ok": True, "service": "brain", "version": "0.3.0"})
        else:
            self._send_json(404, {"ok": False, "error": "not_found"})

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode() if content_length > 0 else "{}"

        if self.path == "/v0/brain":
            try:
                req = json.loads(body)
                user_input = req.get("input", "").strip()
                if not user_input:
                    self._send_json(400, {"ok": False, "error": "empty input"})
                    return

                print(f"[brain] Query: {user_input[:80]}")
                start = time.time()
                result = brain_instance.query(user_input)
                elapsed = time.time() - start
                result["ok"] = True
                result["latency_ms"] = int(elapsed * 1000)
                print(f"[brain] Done in {elapsed:.1f}s: {result.get('text', '')[:80]}")
                self._send_json(200, result)

            except Exception as e:
                traceback.print_exc()
                self._send_json(500, {"ok": False, "text": f"Brain error: {e}", "widgets": []})

        elif self.path == "/v0/brain/proactive":
            try:
                context = json.loads(body) if body else {}
                print(f"[brain] Proactive check")
                result = brain_instance.proactive(context)
                result["ok"] = True
                self._send_json(200, result)
            except Exception as e:
                traceback.print_exc()
                self._send_json(500, {"ok": False, "has_insight": False, "error": str(e)})

        elif self.path == "/v0/brain/dashboard":
            try:
                context = json.loads(body) if body else {}
                print(f"[brain] Dashboard request for {context.get('name', 'unknown')}")
                start = time.time()
                result = brain_instance.dashboard(context)
                elapsed = time.time() - start
                result["ok"] = True
                result["latency_ms"] = int(elapsed * 1000)
                print(f"[brain] Dashboard done in {elapsed:.1f}s")
                self._send_json(200, result)
            except Exception as e:
                traceback.print_exc()
                self._send_json(500, {"ok": False, "error": str(e)})
        else:
            self._send_json(404, {"ok": False, "error": "not_found"})


def main():
    global brain_instance

    port = DEFAULT_PORT
    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--port" and i + 1 < len(sys.argv) - 1:
            port = int(sys.argv[i + 2])
        elif arg == "--model" and i + 1 < len(sys.argv) - 1:
            global CLAUDE_MODEL
            CLAUDE_MODEL = sys.argv[i + 2]

    # Ensure unbuffered output for logging
    sys.stdout = os.fdopen(sys.stdout.fileno(), 'w', buffering=1)

    print(f"[brain] AetherOS Brain Server v0.3")
    print(f"[brain] Claude model: {CLAUDE_MODEL}")

    # Verify claude CLI is available
    try:
        result = subprocess.run(["claude", "--version"], capture_output=True, text=True, timeout=5)
        print(f"[brain] Claude CLI: {result.stdout.strip()}")
    except Exception:
        print("[brain] WARNING: claude CLI not found! Brain queries will fail.")

    brain_instance = Brain()

    server = HTTPServer(("0.0.0.0", port), BrainHandler)
    print(f"[brain] Listening on tcp://0.0.0.0:{port}")
    print(f"[brain] Ready for queries.")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[brain] Shutting down.")
        server.shutdown()


if __name__ == "__main__":
    main()
