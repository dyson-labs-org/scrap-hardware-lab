import argparse
import json
import time
from typing import Any, Dict, Optional

from ..common.crypto import TAG_PROOF, load_schnorr_engine, sha256, tagged_hash
from ..common.messages import (
    MSG_PROOF,
    MSG_TASK_ACCEPT,
    MSG_TASK_REJECT,
    MSG_TASK_REQUEST,
    request_hash,
    task_hash_for_signature,
    with_header,
)
from ..common.replay_cache import ReplayCache
from ..common.token import CapabilityToken
from ..common.util import b64decode, canonical_json, hex_to_bytes
from ..transport.udp import bind_socket, send_message


def log(event: str, **fields: Any) -> None:
    record = {"ts": int(time.time()), "event": event, **fields}
    print(json.dumps(record, sort_keys=True))


def load_json(path: str) -> Dict[str, Any]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def read_revocations(path: Optional[str]) -> set[str]:
    if not path:
        return set()
    try:
        with open(path, "r", encoding="utf-8") as handle:
            data = json.load(handle)
        return set(str(x) for x in data)
    except Exception:
        return set()


def main() -> None:
    parser = argparse.ArgumentParser(description="SCRAP executor (demo)")
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--port", type=int, default=7227)
    parser.add_argument("--keys", required=True)
    parser.add_argument("--policy", required=True)
    parser.add_argument("--allow-mock-signatures", action="store_true")
    args = parser.parse_args()

    keys = load_json(args.keys)
    policy = load_json(args.policy)

    node_id = policy.get("node_id")
    if not node_id:
        raise SystemExit("policy requires node_id")

    operator_pubkey = hex_to_bytes(keys["operator_pubkey"])
    executor_pubkey = keys.get("executor_pubkey")
    executor_privkey = keys.get("executor_privkey")

    allow_mock = bool(policy.get("allow_mock_signatures", False) or args.allow_mock_signatures)
    require_commander_sig = bool(policy.get("require_commander_sig", False))
    revocation_list_path = policy.get("revocation_list_path")

    replay_cache = None
    if policy.get("replay_cache_path"):
        replay_cache = ReplayCache(policy["replay_cache_path"])

    execute_delay = int(policy.get("execute_delay_sec", 1))
    socket = bind_socket(args.bind, args.port)
    log("executor_started", bind=args.bind, port=args.port, node_id=node_id)

    engine = load_schnorr_engine()

    while True:
        message, addr = socket.recvfrom(65535)
        try:
            payload = json.loads(message.decode("utf-8"))
        except Exception:
            log("invalid_json", source=str(addr))
            continue

        if payload.get("message_type") != MSG_TASK_REQUEST:
            log("unexpected_message", source=str(addr), message_type=payload.get("message_type"))
            continue

        task_id = payload.get("task_id")
        requested_capability = payload.get("requested_capability")
        commander_pubkey_hex = payload.get("commander_pubkey")

        issues = []
        notes = []
        if not task_id:
            issues.append("missing task_id")
        if not requested_capability:
            issues.append("missing requested_capability")
        if not commander_pubkey_hex:
            issues.append("missing commander_pubkey")

        token_b64 = payload.get("capability_token_b64")
        if not token_b64:
            issues.append("missing capability_token_b64")

        token = None
        if token_b64:
            try:
                token = CapabilityToken.from_bytes(b64decode(token_b64))
            except Exception as exc:
                issues.append(f"token parse error: {exc}")

        if token is not None:
            ok, token_issues, token_notes = token.verify(
                now=int(time.time()),
                expected_audience=node_id,
                required_capability=requested_capability,
                operator_pubkey=operator_pubkey,
                replay_cache=replay_cache,
                allow_mock_signatures=allow_mock,
            )
            if not ok:
                issues.extend(token_issues)
            notes.extend(token_notes)

            if commander_pubkey_hex and token.subject != commander_pubkey_hex:
                issues.append("token subject does not match commander_pubkey")

            revoked = set(read_revocations(revocation_list_path))

            token_id_hex = token.token_id.hex()
            candidates = {token_id_hex}

            # If the token object also exposes a string token id, honor it too.
            # (This makes revocation lists robust across token encodings.)
            token_id_str = getattr(token, "token_id_str", None)
            if token_id_str:
                candidates.add(token_id_str)

            # Some implementations store token_id as a string attribute.
            tokid_attr = getattr(token, "token_id", None)
            if isinstance(tokid_attr, str):
                candidates.add(tokid_attr)

            if candidates & revoked:
                issues.append("token revoked")

        # Verify commander signature if required.
        commander_sig_hex = payload.get("commander_signature")
        if require_commander_sig:
            if not commander_sig_hex:
                issues.append("missing commander_signature")
            else:
                msg32 = task_hash_for_signature(payload)
                sig_bytes = hex_to_bytes(commander_sig_hex)
                verified = None
                if commander_pubkey_hex:
                    try:
                        verified = engine.verify(msg32, sig_bytes, hex_to_bytes(commander_pubkey_hex))
                    except Exception:
                        verified = False
                if verified is None:
                    if allow_mock:
                        notes.append("commander signature verification skipped (mock mode)")
                    else:
                        issues.append("commander signature verification unavailable")
                elif not verified:
                    issues.append("commander signature invalid")

        if issues:
            reject = with_header(
                MSG_TASK_REJECT,
                "task_reject",
                {
                    "task_id": task_id,
                    "timestamp": int(time.time()),
                    "reason": "validation_failed",
                    "details": issues,
                    "notes": notes,
                },
            )
            send_message(socket, addr[0], addr[1], reject)
            log("task_rejected", task_id=task_id, issues=issues, notes=notes)
            continue

        # Accept the task and issue a payment hash.
        preimage = sha256(f"preimage:{task_id}:{time.time()}".encode("utf-8"))
        payment_hash = sha256(preimage)
        accept = with_header(
            MSG_TASK_ACCEPT,
            "task_accept",
            {
                "task_id": task_id,
                "timestamp": int(time.time()),
                "in_reply_to": request_hash(payload),
                "estimated_duration_sec": execute_delay,
                "payment_hash": payment_hash.hex(),
                "amount_sats": int(payload.get("max_amount_sats", 0) or 0),
                "executor_pubkey": executor_pubkey,
            },
        )

        if executor_privkey and engine.available:
            msg32 = sha256(canonical_json(accept))
            signature = engine.sign(msg32, hex_to_bytes(executor_privkey))
            accept["executor_signature"] = signature.hex()
        else:
            accept["executor_signature"] = ""
            notes.append("executor signature mocked")

        send_message(socket, addr[0], addr[1], accept)
        log("task_accepted", task_id=task_id, payment_hash=payment_hash.hex(), notes=notes)

        # Simulated execution
        time.sleep(execute_delay)
        output_summary = {
            "task_id": task_id,
            "status": "completed",
            "completed_at": int(time.time()),
        }
        output_hash = sha256(canonical_json(output_summary))
        proof_ts = int(time.time())
        proof_msg = token.token_id + payment_hash + output_hash + proof_ts.to_bytes(4, "big")
        proof_hash = tagged_hash(TAG_PROOF, proof_msg)

        proof = with_header(
            MSG_PROOF,
            "proof_of_execution",
            {
                "task_id": task_id,
                "timestamp": proof_ts,
                "status": "completed",
                "output_hash": output_hash.hex(),
                "proof_hash": proof_hash.hex(),
                "payment_hash": payment_hash.hex(),
            },
        )
        send_message(socket, addr[0], addr[1], proof)
        log("proof_sent", task_id=task_id, proof_hash=proof_hash.hex())


if __name__ == "__main__":
    main()
