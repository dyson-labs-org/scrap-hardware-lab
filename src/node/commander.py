import argparse
import json
import socket
import time
from typing import Any, Dict

from ..common.crypto import load_schnorr_engine
from ..common.messages import (
    MSG_PROOF,
    MSG_TASK_ACCEPT,
    MSG_TASK_REJECT,
    MSG_TASK_REQUEST,
    task_hash_for_signature,
    with_header,
)
from ..common.util import b64encode, hex_to_bytes
from ..transport.udp import send_message


def log(event: str, **fields: Any) -> None:
    record = {"ts": int(time.time()), "event": event, **fields}
    print(json.dumps(record, sort_keys=True))


def load_json(path: str) -> Dict[str, Any]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def main() -> None:
    parser = argparse.ArgumentParser(description="SCRAP commander (demo)")
    parser.add_argument("--target-host", required=True)
    parser.add_argument("--target-port", type=int, default=7227)
    parser.add_argument("--token", required=True)
    parser.add_argument("--keys", required=True)
    parser.add_argument("--task-id", required=True)
    parser.add_argument("--requested-capability", required=True)
    parser.add_argument("--task-type", default="imaging")
    parser.add_argument("--max-amount-sats", type=int, default=22000)
    parser.add_argument("--allow-mock-signatures", action="store_true")
    parser.add_argument("--timeout", type=int, default=15)
    args = parser.parse_args()

    keys = load_json(args.keys)
    commander_pubkey = keys.get("commander_pubkey")
    commander_privkey = keys.get("commander_privkey")
    if not commander_pubkey:
        raise SystemExit("keys require commander_pubkey")

    with open(args.token, "rb") as handle:
        token_bytes = handle.read()

    request = with_header(
        MSG_TASK_REQUEST,
        "task_request",
        {
            "task_id": args.task_id,
            "timestamp": int(time.time()),
            "task_type": args.task_type,
            "requested_capability": args.requested_capability,
            "max_amount_sats": args.max_amount_sats,
            "capability_token_b64": b64encode(token_bytes),
            "commander_pubkey": commander_pubkey,
        },
    )

    engine = load_schnorr_engine()
    if commander_privkey and engine.available:
        msg32 = task_hash_for_signature(request)
        signature = engine.sign(msg32, hex_to_bytes(commander_privkey))
        request["commander_signature"] = signature.hex()
    else:
        if args.allow_mock_signatures:
            request["commander_signature"] = ""
            log("commander_signature_mocked")
        else:
            raise SystemExit("commander signature unavailable (install coincurve or allow mock)")

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    send_message(sock, args.target_host, args.target_port, request)
    log("task_request_sent", task_id=args.task_id, target=args.target_host)

    deadline = time.time() + args.timeout
    while time.time() < deadline:
        sock.settimeout(2)
        try:
            data, addr = sock.recvfrom(65535)
        except socket.timeout:
            continue

        payload = json.loads(data.decode("utf-8"))
        msg_type = payload.get("message_type")
        if msg_type == MSG_TASK_REJECT:
            log("task_rejected", task_id=args.task_id, details=payload.get("details"), notes=payload.get("notes"))
            return
        if msg_type == MSG_TASK_ACCEPT:
            log("task_accepted", task_id=args.task_id, payment_hash=payload.get("payment_hash"))
            continue
        if msg_type == MSG_PROOF:
            log("proof_received", task_id=args.task_id, proof_hash=payload.get("proof_hash"))
            return

    log("timeout_waiting_for_response", task_id=args.task_id)


if __name__ == "__main__":
    main()
