import argparse
import json
import os
import secrets
import time
from typing import Dict, List

from ..common.crypto import TAG_TOKEN, load_schnorr_engine, tagged_hash
from ..common.tlv import encode_bigsize
from ..common.util import bytes_to_hex, hex_to_bytes

TLV_VERSION = 0
TLV_ISSUER = 2
TLV_SUBJECT = 4
TLV_AUDIENCE = 6
TLV_ISSUED_AT = 8
TLV_EXPIRES_AT = 10
TLV_TOKEN_ID = 12
TLV_CAPABILITY = 14
TLV_SIGNATURE = 240
TLV_CONSTRAINT_AFTER = 19


def encode_record(tlv_type: int, value: bytes) -> bytes:
    return encode_bigsize(tlv_type) + encode_bigsize(len(value)) + value


def load_keys(path: str) -> Dict[str, str]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def save_json(path: str, data: Dict[str, str]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, sort_keys=True)


def issue_token(args: argparse.Namespace) -> None:
    keys = load_keys(args.keys)
    operator_pubkey = hex_to_bytes(keys["operator_pubkey"])
    operator_privkey = keys.get("operator_privkey")

    issued_at = int(args.issued_at or time.time())
    expires_at = issued_at + int(args.expires_in)
    token_id = hex_to_bytes(args.token_id) if args.token_id else secrets.token_bytes(16)

    records: List[bytes] = [
        encode_record(TLV_VERSION, (1).to_bytes(1, "big")),
        encode_record(TLV_ISSUER, operator_pubkey),
        encode_record(TLV_SUBJECT, args.subject.encode("utf-8")),
        encode_record(TLV_AUDIENCE, args.audience.encode("utf-8")),
        encode_record(TLV_ISSUED_AT, issued_at.to_bytes(4, "big")),
        encode_record(TLV_EXPIRES_AT, expires_at.to_bytes(4, "big")),
        encode_record(TLV_TOKEN_ID, token_id),
    ]

    for cap in args.capability:
        records.append(encode_record(TLV_CAPABILITY, cap.encode("utf-8")))

    if args.not_before:
        records.append(encode_record(TLV_CONSTRAINT_AFTER, int(args.not_before).to_bytes(4, "big")))

    body = b"".join(records)

    engine = load_schnorr_engine()
    signature = None
    if operator_privkey:
        msg32 = tagged_hash(TAG_TOKEN, body)
        signature = engine.sign(msg32, hex_to_bytes(operator_privkey)) if engine.available else None

    if signature is None:
        if args.allow_mock_signature:
            signature = b"\x00" * 64
        else:
            raise SystemExit("signature unavailable in mock-only mode")

    token = body + encode_record(TLV_SIGNATURE, signature)

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "wb") as handle:
        handle.write(token)

    if args.meta_out:
        meta = {
            "token_id": bytes_to_hex(token_id),
            "issued_at": issued_at,
            "expires_at": expires_at,
            "audience": args.audience,
            "subject": args.subject,
            "capabilities": args.capability,
            "signature_mocked": signature == b"\x00" * 64,
        }
        save_json(args.meta_out, meta)


def revoke_token(args: argparse.Namespace) -> None:
    revoked = []
    if os.path.exists(args.revocation_list):
        with open(args.revocation_list, "r", encoding="utf-8") as handle:
            revoked = json.load(handle)

    token_id = args.token_id
    revoked.append(token_id)

    os.makedirs(os.path.dirname(args.revocation_list), exist_ok=True)
    with open(args.revocation_list, "w", encoding="utf-8") as handle:
        json.dump(sorted(set(revoked)), handle, indent=2)


def main() -> None:
    parser = argparse.ArgumentParser(description="SCRAP operator stub (demo)")
    sub = parser.add_subparsers(dest="cmd", required=True)

    issue = sub.add_parser("issue-token", help="issue capability token")
    issue.add_argument("--keys", required=True)
    issue.add_argument("--out", required=True)
    issue.add_argument("--meta-out")
    issue.add_argument("--subject", required=True)
    issue.add_argument("--audience", required=True)
    issue.add_argument("--capability", action="append", required=True)
    issue.add_argument("--expires-in", type=int, default=3600)
    issue.add_argument("--issued-at", type=int)
    issue.add_argument("--token-id")
    issue.add_argument("--not-before", type=int)
    issue.add_argument("--allow-mock-signature", action="store_true")

    revoke = sub.add_parser("revoke", help="revoke token id in list")
    revoke.add_argument("--revocation-list", required=True)
    revoke.add_argument("--token-id", required=True)

    args = parser.parse_args()
    if args.cmd == "issue-token":
        issue_token(args)
    elif args.cmd == "revoke":
        revoke_token(args)


if __name__ == "__main__":
    main()
