import base64
import json
from typing import Any, Dict


def b64encode(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")


def b64decode(data: str) -> bytes:
    return base64.b64decode(data.encode("ascii"))


def canonical_json(obj: Dict[str, Any]) -> bytes:
    """Deterministic JSON for hashing/signing."""
    return json.dumps(obj, sort_keys=True, separators=(",", ":")).encode("utf-8")


def hex_to_bytes(value: str) -> bytes:
    value = value.strip().lower()
    if value.startswith("0x"):
        value = value[2:]
    return bytes.fromhex(value)


def bytes_to_hex(value: bytes) -> str:
    return value.hex()
