# __VULTS_MARKER__: local session observer; never returns an approval or changes tool arguments.
import atexit
import json
import math
import os
import queue
import subprocess
import sys
import threading
import time

_RELAY = __VULTS_RELAY__
_queue = queue.Queue(maxsize=128)
_worker = None
_lock = threading.Lock()
_api_time = {}
_seen_api = set()


def _drain(pending, stop):
    while not stop.is_set():
        try:
            event, payload = pending.get(timeout=0.2)
        except queue.Empty:
            continue
        try:
            subprocess.run([_RELAY, "--agent", "hermes", event], input=payload,
                           text=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           timeout=2, check=False)
        except Exception:
            pass
        finally:
            pending.task_done()


def _flush():
    deadline = time.monotonic() + 0.4
    while _queue.unfinished_tasks and time.monotonic() < deadline:
        time.sleep(0.01)


def _send(event, session_id="", tool_name="", args=None, tool_call_id="", **kwargs):
    if not session_id or os.environ.get("VULTS_RUN") or os.environ.get("VULTS_ZECA"):
        return
    try:
        allowed = ("command", "path", "file_path", "cwd", "workdir", "directory",
                   "pattern", "query", "url", "old_string", "new_string", "patch", "mode")
        tool_input = {k: v[:16000] for k, v in (args or {}).items()
                      if k in allowed and isinstance(v, str)}
        terminal = sys.modules.get("tools.terminal_tool")
        recorded = getattr(terminal, "get_session_cwd", lambda _: None)(kwargs.get("task_id") or session_id)
        cwd = recorded or kwargs.get("cwd") or os.environ.get("TERMINAL_CWD") or os.getcwd()
        payload = {"session_id": str(session_id), "cwd": os.path.abspath(os.path.expanduser(cwd)),
                   "tool_name": tool_name, "tool_input": tool_input,
                   "tool_use_id": str(tool_call_id or "")}
        if event == "SessionMetrics":
            for key in ("elapsed_ms", "context_tokens", "context_limit"):
                value = kwargs.get(key)
                if isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= 2**53:
                    payload[key] = value
        _queue.put_nowait((event, json.dumps(payload)))
    except Exception:
        pass


def _start(**kwargs):
    _send("SessionStart", **kwargs)
    _send("UserPromptSubmit", **kwargs)


def _done(failed=False, **kwargs):
    _send("StopFailure" if failed else "Stop", **kwargs)


def _post(status="", failed=False, **kwargs):
    _send("PostToolUseFailure" if failed or status == "error" else "PostToolUse", **kwargs)


def _sub(event, parent_session_id="", session_id="", **kwargs):
    _send(event, session_id=parent_session_id or session_id, **kwargs)


def _finalize(platform="", **kwargs):
    # Desktop finalizes evicted/idle runtimes too, without closing its conversation.
    # Native finalize does not forward the teardown reason; it is not proof of UI closure.
    _send("Stop" if str(platform).strip().lower() == "desktop" else "SessionEnd", **kwargs)


def _reset(session_id="", old_session_id="", new_session_id="", **kwargs):
    current = new_session_id or session_id
    if old_session_id and old_session_id != current:
        _send("SessionEnd", session_id=old_session_id, **kwargs)
    if current:
        _send("SessionStart", session_id=current, **kwargs)


def _api(session_id="", usage=None, api_duration=None, context_length=None,
         api_request_id="", task_id="", turn_id="", api_call_count=None, **kwargs):
    if not session_id or os.environ.get("VULTS_RUN") or os.environ.get("VULTS_ZECA"):
        return
    usage = usage if isinstance(usage, dict) else {}
    tokens = usage.get("prompt_tokens")
    with _lock:
        identity = api_request_id or (task_id, turn_id, api_call_count)
        identity = (str(session_id), str(identity))
        if identity not in _seen_api:
            # Bounded cache for long-running gateways; the CLI normally supplies a UUID per call.
            if len(_seen_api) >= 4096:
                _seen_api.clear()
            _seen_api.add(identity)
            if isinstance(api_duration, (int, float)) and not isinstance(api_duration, bool) and math.isfinite(api_duration) and 0 <= api_duration <= 86400:
                if len(_api_time) >= 128 and session_id not in _api_time:
                    _api_time.pop(next(iter(_api_time)))
                _api_time[session_id] = _api_time.get(session_id, 0) + round(api_duration * 1000)
        elapsed = _api_time.get(session_id)
    _send("SessionMetrics", session_id=session_id, elapsed_ms=elapsed,
          context_tokens=tokens, context_limit=context_length)


def register(ctx):
    global _worker, _queue
    with _lock:
        pending = _queue = queue.Queue(maxsize=128)
        stop = threading.Event()
        _worker = threading.Thread(target=_drain, args=(pending, stop), daemon=True, name="vults-hermes-relay")
        _worker.start()
        atexit.register(_flush)
    if callable(getattr(ctx, "on_unload", None)):
        ctx.on_unload(stop.set)
    ctx.register_hook("on_session_start", _start)
    ctx.register_hook("on_session_end", _done)
    ctx.register_hook("on_session_finalize", _finalize)
    ctx.register_hook("on_session_reset", _reset)
    ctx.register_hook("on_session_delete", lambda **kw: _send("SessionEnd", **kw))
    ctx.register_hook("pre_tool_call", lambda **kw: _send("PreToolUse", **kw))
    ctx.register_hook("post_tool_call", _post)
    ctx.register_hook("post_api_request", _api)
    ctx.register_hook("subagent_start", lambda **kw: _sub("SubagentStart", **kw))
    ctx.register_hook("subagent_stop", lambda **kw: _sub("SubagentStop", **kw))
