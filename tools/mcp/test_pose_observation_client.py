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
