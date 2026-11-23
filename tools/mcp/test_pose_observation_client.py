"""Unit tests for MCP image-block parsing; fixtures are not native capture evidence."""

from __future__ import annotations

import base64
from typing import Any

import pytest
from pose_observation_client import McpStdioClient

PNG_FIXTURE = base64.b64encode(
    b"\x89PNG\r\n\x1a\nfixture-pixel-bytes"
).decode("ascii")


class FakeClient(McpStdioClient):
    def __init__(self, result: dict[str, Any]) -> None:
        self.result = result

    def call_tool(
        self, name: str, arguments: dict[str, Any], timeout: float = 35
    ) -> dict[str, Any]:
        del name, arguments, timeout
        return self.result


def client_with_result(result: dict[str, Any]) -> FakeClient:
    return FakeClient(result)


def test_capture_view_decodes_standard_mcp_image_content() -> None:
    client = client_with_result(
        {
            "content": [
                {
                    "type": "text",
                    "text": '{"capture_id":"capture-1","view":"front","mime":"image/png"}',
                },
                {"type": "image", "mimeType": "image/png", "data": PNG_FIXTURE},
            ]
        }
    )
    metadata, mime, raw = client.capture_view("capture-1", "front")
    assert metadata["capture_id"] == "capture-1"
    assert mime == "image/png"
    assert raw.startswith(b"\x89PNG\r\n\x1a\n")


def test_capture_view_rejects_metadata_mismatch_and_unknown_camera() -> None:
    result = {
        "content": [
            {"type": "text", "text": '{"capture_id":"other","view":"front","mime":"image/png"}'},
            {"type": "image", "mimeType": "image/png", "data": PNG_FIXTURE},
        ]
    }
    client = client_with_result(result)
    with pytest.raises(ValueError, match="unsupported observation camera"):
        client.capture_view("capture-1", "desktop")
    with pytest.raises(ValueError, match="does not correlate"):
        client.capture_view("capture-1", "front")


def test_observe_pose_requires_unique_supported_views_before_rpc() -> None:
    client = client_with_result({"structuredContent": {}})
    with pytest.raises(ValueError, match="unique camera names"):
        client.observe_pose(
            draft_id="draft-1",
            revision=1,
            timestamp_seconds=0.5,
            views=["front", "front"],
        )
    with pytest.raises(ValueError, match="unsupported observation camera"):
        client.observe_pose(
            draft_id="draft-1",
            revision=1,
            timestamp_seconds=0.5,
            views=["desktop"],
        )


def test_close_waits_for_graceful_mcp_shutdown_before_terminating() -> None:
    events: list[str] = []

    class Stdin:
        closed = False

        def close(self) -> None:
            events.append("stdin-close")
            self.closed = True

    class Process:
        def poll(self) -> None:
            return None

        def wait(self, timeout: float | None = None) -> int:
            del timeout
            events.append("wait")
            return 0

        def terminate(self) -> None:
            events.append("terminate")

        def kill(self) -> None:
            events.append("kill")

    class ReaderThread:
        def join(self, timeout: float | None = None) -> None:
            del timeout

    class Transcript:
        def close(self) -> None:
            events.append("transcript-close")

    client = McpStdioClient.__new__(McpStdioClient)
    client.stdin = Stdin()  # type: ignore[assignment]  # ty: ignore[invalid-assignment]
    client.process = Process()  # type: ignore[assignment]  # ty: ignore[invalid-assignment]
    client.reader_thread = ReaderThread()  # type: ignore[assignment]  # ty: ignore[invalid-assignment]
    client.transcript = Transcript()  # type: ignore[assignment]  # ty: ignore[invalid-assignment]
    client.close()

    assert events[:2] == ["stdin-close", "wait"]
    assert "terminate" not in events
