import argparse
import json
import os
import socket
import time
import urllib.error
import urllib.request
from typing import Any, Dict, Optional

from ..settlement import (
    SettlementError,
    SettlementRecord,
    SettlementState,
    SettlementStore,
    compute_payment_hash,
    compute_proof_hash,
)
from ..transport.udp import bind_socket, recv_message, send_message


def unix_ts() -> int:
    return int(time.time())


def log(event: str, **fields: Any) -> None:
    record = {"ts": unix_ts(), "event": event, **fields}
    print(json.dumps(record, sort_keys=True))


def load_json(path: str) -> Dict[str, Any]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def sha256_hex(parts: list[str]) -> str:
    import hashlib

    hasher = hashlib.sha256()
    for part in parts:
        hasher.update(part.encode("utf-8"))
    return hasher.hexdigest()


def derive_demo_correlation_id(task_id: str, token_id: str) -> str:
    seed = f"{task_id}:{token_id}"
    return sha256_hex([seed])


class BtcpayClient:
    def create_invoice(self, usd_amount: float, metadata: Dict[str, str]) -> Dict[str, Any]:
        raise NotImplementedError

    def get_invoice(self, invoice_id: str) -> Dict[str, Any]:
        raise NotImplementedError


class FakeBtcpayClient(BtcpayClient):
    def __init__(self, auto_pay_after_sec: int = 2) -> None:
        self.auto_pay_after_sec = auto_pay_after_sec
        self.invoices: Dict[str, Dict[str, Any]] = {}

    def create_invoice(self, usd_amount: float, metadata: Dict[str, str]) -> Dict[str, Any]:
        invoice_id = sha256_hex(
            [
                f"{usd_amount:.2f}",
                metadata.get("task_id", ""),
                metadata.get("token_id", ""),
                "fake",
            ]
        )[:32]
        now = unix_ts()
        invoice_url = f"https://fake.btcpay.local/i/{invoice_id}"
        invoice = {
            "id": invoice_id,
            "status": "New",
            "amount": usd_amount,
            "currency": "USD",
            "metadata": metadata,
            "created_at": now,
            "paid_at": None,
            "checkoutLink": invoice_url,
        }
        self.invoices[invoice_id] = invoice
        return {
            "invoice_id": invoice_id,
            "invoice_url": invoice_url,
            "status": "New",
        }

    def get_invoice(self, invoice_id: str) -> Dict[str, Any]:
        invoice = self.invoices.get(invoice_id)
        if invoice is None:
            raise SettlementError("invoice_missing", f"unknown invoice {invoice_id}")
        now = unix_ts()
        if (
            invoice["status"] == "New"
            and self.auto_pay_after_sec >= 0
            and now - int(invoice["created_at"]) >= self.auto_pay_after_sec
        ):
            invoice["status"] = "Paid"
            invoice["paid_at"] = now
        return {
            "status": invoice["status"],
            "paid_at": invoice["paid_at"],
            "amount": invoice["amount"],
            "currency": invoice["currency"],
            "metadata": invoice.get("metadata", {}),
        }


class RealBtcpayClient(BtcpayClient):
    def __init__(self, api_base: str, api_key: str, store_id: str, timeout: int = 10) -> None:
        self.api_base = api_base.rstrip("/")
        self.api_key = api_key
        self.store_id = store_id
        self.timeout = timeout

    def _request(self, method: str, path: str, body: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        url = f"{self.api_base}{path}"
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("Authorization", f"token {self.api_key}")
        req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read()
        except urllib.error.HTTPError as err:
            details = err.read().decode("utf-8", errors="replace")
            raise SettlementError("btcpay_http_error", f"{err.code} {err.reason}: {details}") from err
        except urllib.error.URLError as err:
            raise SettlementError("btcpay_url_error", str(err)) from err
        if not raw:
            return {}
        return json.loads(raw.decode("utf-8"))

    def create_invoice(self, usd_amount: float, metadata: Dict[str, str]) -> Dict[str, Any]:
        payload = {
            "amount": usd_amount,
            "currency": "USD",
            "metadata": metadata,
        }
        data = self._request("POST", f"/api/v1/stores/{self.store_id}/invoices", payload)
        invoice_id = data.get("id") or data.get("invoiceId")
        invoice_url = data.get("checkoutLink") or data.get("url")
        if not invoice_id:
            raise SettlementError("btcpay_missing_invoice_id", "missing invoice id in response")
        if not invoice_url:
            invoice_url = f"{self.api_base}/i/{invoice_id}"
        return {
            "invoice_id": invoice_id,
            "invoice_url": invoice_url,
            "status": data.get("status", "New"),
        }

    def get_invoice(self, invoice_id: str) -> Dict[str, Any]:
        data = self._request("GET", f"/api/v1/stores/{self.store_id}/invoices/{invoice_id}")
        return {
            "status": data.get("status"),
            "paid_at": data.get("paidAt") or data.get("paidAtUnix"),
            "amount": data.get("amount"),
            "currency": data.get("currency"),
            "metadata": data.get("metadata", {}),
            "additional_status": data.get("additionalStatus"),
        }


PAID_STATUSES = {"paid", "confirmed", "complete", "settled"}


def invoice_paid_for_demo(invoice: Dict[str, Any]) -> bool:
    status = str(invoice.get("status") or "").lower()
    additional = str(invoice.get("additional_status") or "").lower()
    return status in PAID_STATUSES or additional in PAID_STATUSES


def resolve_btcpay_config(args: argparse.Namespace) -> Dict[str, str]:
    config: Dict[str, str] = {}
    if args.btcpay_config:
        config.update(load_json(args.btcpay_config))
    config_env = {
        "api_base": os.environ.get("BTCPAY_URL"),
        "api_key": os.environ.get("BTCPAY_API_KEY"),
        "store_id": os.environ.get("BTCPAY_STORE_ID"),
    }
    for key, value in config_env.items():
        if value:
            config[key] = value
    if args.btcpay_url:
        config["api_base"] = args.btcpay_url
    if args.btcpay_api_key:
        config["api_key"] = args.btcpay_api_key
    if args.btcpay_store_id:
        config["store_id"] = args.btcpay_store_id
    return config


def ensure_commander_pubkey(token: Dict[str, Any], keys_path: Optional[str], override: Optional[str]) -> str:
    commander_pubkey = override or token.get("subject")
    if keys_path and os.path.exists(keys_path):
        keys = load_json(keys_path)
        keys_pubkey = keys.get("commander_pubkey")
        if keys_pubkey and commander_pubkey and commander_pubkey != keys_pubkey:
            raise SystemExit("commander_pubkey does not match keys.json")
        commander_pubkey = commander_pubkey or keys_pubkey
    if not commander_pubkey:
        raise SystemExit("missing commander_pubkey (token subject or keys.json)")
    if token.get("subject") and commander_pubkey != token.get("subject"):
        raise SystemExit("token subject does not match commander_pubkey")
    return commander_pubkey


def build_task_request(
    task_id: str,
    requested_capability: str,
    token: Dict[str, Any],
    commander_pubkey: str,
    correlation_id: str,
    max_amount_sats: int,
    timeout_blocks: int,
) -> Dict[str, Any]:
    return {
        "version": 1,
        "type": "task_request",
        "task_id": task_id,
        "requested_capability": requested_capability,
        "payment_terms": {
            "max_amount_sats": max_amount_sats,
            "timeout_blocks": timeout_blocks,
        },
        "correlation_id": correlation_id,
        "token": token,
        "commander_pubkey": commander_pubkey,
        "commander_signature": "mock",
    }


def build_payment_lock(
    task_id: str,
    correlation_id: str,
    payment_hash: str,
    amount_sats: int,
    timeout_blocks: int,
) -> Dict[str, Any]:
    return {
        "type": "payment_lock",
        "task_id": task_id,
        "correlation_id": correlation_id,
        "payment_hash": payment_hash,
        "amount_sats": amount_sats,
        "timeout_blocks": timeout_blocks,
        "timestamp": unix_ts(),
    }


def wait_for_payment(
    client: BtcpayClient,
    invoice_id: str,
    poll_interval: int,
    timeout_sec: int,
) -> Dict[str, Any]:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        invoice = client.get_invoice(invoice_id)
        if invoice_paid_for_demo(invoice):
            return invoice
        time.sleep(poll_interval)
    raise SettlementError("invoice_timeout", "invoice not paid before timeout")


def wait_for_proof(
    sock: socket.socket,
    expected_task_id: str,
    expected_payment_hash: str,
    expected_proof_hash: str,
    timeout_sec: int,
) -> None:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        try:
            payload, _ = recv_message(sock, timeout=2.0)
        except socket.timeout:
            continue
        msg_type = payload.get("type")
        if msg_type == "task_accepted":
            payment_hash = payload.get("payment_hash")
            if payment_hash != expected_payment_hash:
                raise SettlementError("payment_hash_mismatch", "task_accepted payment_hash mismatch")
            log("task_accepted", payment_hash=payment_hash)
            continue
        if msg_type == "proof":
            proof_hash = payload.get("proof_hash")
            if payload.get("task_id") != expected_task_id:
                raise SettlementError("task_id_mismatch", "proof task_id mismatch")
            if proof_hash != expected_proof_hash:
                raise SettlementError("proof_hash_mismatch", "proof hash mismatch")
            log("proof_received", proof_hash=proof_hash)
            return
        if msg_type == "task_rejected":
            details = payload.get("details")
            raise SettlementError("task_rejected", f"task rejected: {details}")
        if msg_type == "payment_claim":
            log("payment_claim_received", payment_hash=payload.get("payment_hash"))
    raise SettlementError("proof_timeout", "proof not received before timeout")


def main() -> None:
    parser = argparse.ArgumentParser(description="SCRAP settlement bridge (BTCPay)")
    parser.add_argument("--usd", type=float, required=True)
    parser.add_argument("--task-id")
    parser.add_argument("--token-id")
    parser.add_argument("--token", default="demo/config/token.json")
    parser.add_argument("--keys", default="demo/config/keys.json")
    parser.add_argument("--requested-capability")
    parser.add_argument("--target-host", required=True)
    parser.add_argument("--target-port", type=int, default=7227)
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--bind-port", type=int, default=0)
    parser.add_argument("--max-amount-sats", type=int, default=25000)
    parser.add_argument("--timeout-blocks", type=int, default=144)
    parser.add_argument("--poll-interval", type=int, default=2)
    parser.add_argument("--invoice-timeout", type=int, default=900)
    parser.add_argument("--exec-timeout", type=int, default=60)
    parser.add_argument("--settlement-store", default="demo/runtime/settlement.json")
    parser.add_argument("--btcpay-config")
    parser.add_argument("--btcpay-url")
    parser.add_argument("--btcpay-api-key")
    parser.add_argument("--btcpay-store-id")
    parser.add_argument("--fake", action="store_true")
    parser.add_argument("--real", action="store_true")
    parser.add_argument("--fake-auto-pay-after", type=int, default=2)
    args = parser.parse_args()

    if args.fake and args.real:
        raise SystemExit("choose only one: --fake or --real")

    token = load_json(args.token)
    token_id = args.token_id or token.get("token_id")
    if not token_id:
        raise SystemExit("token_id missing (pass --token-id or include token_id in token json)")
    if args.token_id and token.get("token_id") and args.token_id != token.get("token_id"):
        raise SystemExit("token_id does not match token json")

    task_id = args.task_id or sha256_hex([str(unix_ts()), token_id])[:32]
    commander_pubkey = ensure_commander_pubkey(token, args.keys, None)
    requested_capability = args.requested_capability or token.get("capability") or "telemetry.read"

    correlation_id = derive_demo_correlation_id(task_id, token_id)
    payment_hash = compute_payment_hash(task_id, token_id)
    proof_hash = compute_proof_hash(task_id, payment_hash)

    metadata = {
        "task_id": task_id,
        "token_id": token_id,
        "payment_hash": payment_hash,
        "proof_hash": proof_hash,
    }

    store = SettlementStore(args.settlement_store)

    if args.real:
        cfg = resolve_btcpay_config(args)
        missing = [k for k in ("api_base", "api_key", "store_id") if not cfg.get(k)]
        if missing:
            raise SystemExit(f"missing BTCPay config: {', '.join(missing)}")
        client: BtcpayClient = RealBtcpayClient(
            api_base=cfg["api_base"],
            api_key=cfg["api_key"],
            store_id=cfg["store_id"],
        )
    else:
        client = FakeBtcpayClient(auto_pay_after_sec=args.fake_auto_pay_after)

    invoice = client.create_invoice(args.usd, metadata)
    invoice_id = invoice["invoice_id"]
    invoice_url = invoice["invoice_url"]

    record = SettlementRecord(
        task_id=task_id,
        token_id=token_id,
        payment_hash=payment_hash,
        proof_hash=proof_hash,
        btcpay_invoice_id=invoice_id,
        btcpay_invoice_url=invoice_url,
        state=SettlementState.Requested,
        requested_at=unix_ts(),
    )
    store.upsert(record)

    print(f"INVOICE_URL {invoice_url}")
    log("invoice_created", task_id=task_id, invoice_id=invoice_id, usd_amount=args.usd)

    sock = bind_socket(args.bind, args.bind_port)
    task_request = build_task_request(
        task_id=task_id,
        requested_capability=requested_capability,
        token=token,
        commander_pubkey=commander_pubkey,
        correlation_id=correlation_id,
        max_amount_sats=args.max_amount_sats,
        timeout_blocks=args.timeout_blocks,
    )
    send_message(sock, args.target_host, args.target_port, task_request)
    log("task_request_sent", task_id=task_id, target=args.target_host)

    try:
        invoice_status = wait_for_payment(client, invoice_id, args.poll_interval, args.invoice_timeout)
        record.mark_locked(unix_ts())
        store.upsert(record)
        log("payment_locked", task_id=task_id, status=invoice_status.get("status"))

        lock = build_payment_lock(
            task_id=task_id,
            correlation_id=correlation_id,
            payment_hash=payment_hash,
            amount_sats=args.max_amount_sats,
            timeout_blocks=args.timeout_blocks,
        )
        send_message(sock, args.target_host, args.target_port, lock)
        log("payment_lock_sent", task_id=task_id, payment_hash=payment_hash)

        wait_for_proof(
            sock=sock,
            expected_task_id=task_id,
            expected_payment_hash=payment_hash,
            expected_proof_hash=proof_hash,
            timeout_sec=args.exec_timeout,
        )
        record.mark_claimed(proof_hash, unix_ts())
        store.upsert(record)

        claimed_at = record.claimed_at or unix_ts()
        print(
            "DEMO SUCCESS "
            f"task_id={task_id} invoice_id={invoice_id} usd_amount={args.usd} "
            f"payment_hash={payment_hash} proof_hash={proof_hash} claimed_at={claimed_at}"
        )
    except SettlementError as err:
        record.last_error = f"{err.code}: {err.message}"
        store.upsert(record)
        log("settlement_error", code=err.code, message=err.message)
        raise SystemExit(2) from err


if __name__ == "__main__":
    main()
