from typing import Any, Dict

from .crypto import TAG_TASK, sha256, tagged_hash
from .util import canonical_json

MSG_TASK_REQUEST = 0x01
MSG_TASK_ACCEPT = 0x02
MSG_TASK_REJECT = 0x03
MSG_PROOF = 0x04


def with_header(message_type: int, message_name: str, body: Dict[str, Any]) -> Dict[str, Any]:
    payload = dict(body)
    payload["message_type"] = message_type
    payload["message_name"] = message_name
    return payload


def request_hash(request: Dict[str, Any]) -> str:
    # Demo assumption: hash canonical JSON without commander signature fields.
    base = dict(request)
    base.pop("commander_signature", None)
    base.pop("message_name", None)
    return sha256(canonical_json(base)).hex()


def task_hash_for_signature(request: Dict[str, Any]) -> bytes:
    # Demo assumption: tag hash of canonical request JSON (sans signature).
    base = dict(request)
    base.pop("commander_signature", None)
    base.pop("message_name", None)
    return tagged_hash(TAG_TASK, canonical_json(base))
