"""Real MCP stdio client helpers for bounded pose observations and image blocks."""

from __future__ import annotations

import base64
import hashlib
import json
import os
import queue
import subprocess
import threading
import time
from pathlib import Path
from typing import Any

VIEWS = ("front", "back", "left", "right", "top", "bottom")
IMAGE_LIMITS = {"image/png": 1024 * 1024, "image/jpeg": 256 * 1024}


class McpStdioClient:
    """Small request-correlated client that preserves raw MCP content blocks."""

    def __init__(self, binary: Path, endpoint: str, transcript: Path) -> None:
        self.process = subprocess.Popen(
            [str(binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=dict(os.environ, M2M_MCP_IPC_ENDPOINT=endpoint),
        )
        if self.process.stdin is None or self.process.stdout is None:
            raise RuntimeError("MCP sidecar stdio pipes were not created")
        self.stdin = self.process.stdin
        self.lines: queue.Queue[str] = queue.Queue()
        self.reader_thread = threading.Thread(
            target=self._read_lines, args=(self.process.stdout,), daemon=True
        )
        self.reader_thread.start()
        self.request_id = 0
        self.transcript = transcript.open("w", encoding="utf-8")
        self.send(
            "initialize",
            {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "m2m030b-pose-observation", "version": "1"},
            },
        )
        self.stdin.write(
            json.dumps(
                {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
            )
            + "\n"
        )
        self.stdin.flush()

    def _record(self, entry: dict[str, Any]) -> None:
        self.transcript.write(json.dumps(entry, separators=(",", ":")) + "\n")
        self.transcript.flush()

    def _read_lines(self, stream: Any) -> None:
        for line in stream:
            self.lines.put(line.rstrip("\n"))

    def send(
        self, method: str, params: dict[str, Any], timeout: float = 35
    ) -> dict[str, Any]:
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params,
        }
        self._record({"request": request})
        self.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.stdin.flush()
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                message = json.loads(
                    self.lines.get(timeout=max(0.01, deadline - time.monotonic()))
                )
            except queue.Empty as error:
                raise TimeoutError(f"no response to {method}") from error
            if message.get("id") == self.request_id:
                self._record({"response": message})
                return message
            self._record({"notification": message})
        raise TimeoutError(f"no correlated response to {method}")

    def call_tool(
        self, name: str, arguments: dict[str, Any], timeout: float = 35
    ) -> dict[str, Any]:
        response = self.send(
            "tools/call",
            {"name": name, "arguments": arguments},
            timeout=timeout,
        )
        result = response.get("result", {})
        if result.get("isError"):
            raise RuntimeError(f"{name} returned an MCP tool error: {result}")
        return result

    def observe_pose(
        self,
        *,
        draft_id: str,
        revision: int,
        timestamp_seconds: float,
        views: list[str],
        focus_bone_id: str | None = None,
    ) -> dict[str, Any]:
        if not 1 <= len(views) <= len(VIEWS) or len(set(views)) != len(views):
            raise ValueError("views must contain 1..6 unique camera names")
        if any(view not in VIEWS for view in views):
            raise ValueError("unsupported observation camera")
        result = self.call_tool(
            "animation_observe_pose",
            {
                "draft_id": draft_id,
                "expected_revision": revision,
                "timestamp_seconds": timestamp_seconds,
                "views": views,
                "focus_bone_id": focus_bone_id,
            },
        )
        structured = result.get("structuredContent")
        if not isinstance(structured, dict):
            raise TypeError("pose observation did not return structured capture metadata")
        return structured

    def import_draft(self, document: dict[str, Any]) -> dict[str, Any]:
        """Create/replace a complete validated animation draft in one MCP call.

        Build channels from the live ``skeleton_get_catalog`` result and compose
        each authored delta with that bone's local rest transform before calling
        this method. The Rust authoring bridge validates the document against
        the active rig and applies it atomically; no per-bone request loop is
        needed.
        """
        result = self.call_tool("animation_import_draft", {"document": document})
        structured = result.get("structuredContent")
        if not isinstance(structured, dict):
            raise TypeError("draft import did not return structured revision metadata")
        return structured

    def capture_view(self, capture_id: str, view: str) -> tuple[dict[str, Any], str, bytes]:
        if view not in VIEWS:
            raise ValueError("unsupported observation camera")
        result = self.call_tool(
            "animation_capture_view", {"capture_id": capture_id, "view": view}
        )
        content = result.get("content")
        if not isinstance(content, list):
            raise TypeError("MCP image result has no content list")
        metadata = next(
            (
                json.loads(item["text"])
                for item in content
                if isinstance(item, dict) and isinstance(item.get("text"), str)
            ),
            None,
        )
        image = next(
            (
                item
                for item in content
                if isinstance(item, dict) and item.get("type") == "image"
            ),
            None,
        )
        if not isinstance(metadata, dict) or not isinstance(image, dict):
            raise TypeError("MCP response must contain metadata and an image content block")
        mime = image.get("mimeType")
        if not isinstance(mime, str) or mime not in IMAGE_LIMITS:
            raise ValueError(f"unsupported MCP image MIME type: {mime!r}")
        encoded = image.get("data")
        if not isinstance(encoded, str):
            raise TypeError("MCP image block has no base64 data")
        raw = base64.b64decode(encoded, validate=True)
        if not raw or len(raw) > IMAGE_LIMITS[mime]:
            raise ValueError("MCP image bytes are empty or exceed the MIME limit")
        if metadata.get("capture_id") != capture_id or metadata.get("view") != view:
            raise ValueError("image metadata does not correlate with the requested capture/view")
        if metadata.get("mime") != mime:
            raise ValueError("MCP image MIME does not match its camera metadata")
        if mime == "image/png" and not raw.startswith(b"\x89PNG\r\n\x1a\n"):
            raise ValueError("MCP image payload is not a PNG stream")
        if mime == "image/jpeg" and not (raw.startswith(b"\xff\xd8") and raw.endswith(b"\xff\xd9")):
            raise ValueError("MCP image payload is not a JPEG stream")
        return metadata, mime, raw

    def save_image(self, directory: Path, capture_id: str, view: str) -> dict[str, Any]:
        metadata, mime, raw = self.capture_view(capture_id, view)
        suffix = ".png" if mime == "image/png" else ".jpg"
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"{capture_id}-{view}{suffix}"
        path.write_bytes(raw)
        return {
            "path": str(path),
            "mime": mime,
            "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "metadata": metadata,
        }

    def close(self) -> None:
        if not self.stdin.closed:
            self.stdin.close()
        if self.process.poll() is None:
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
        self.reader_thread.join(timeout=1)
        self.transcript.close()
