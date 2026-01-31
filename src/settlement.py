from __future__ import annotations

import hashlib
import json
import os
from dataclasses import asdict, dataclass
from enum import Enum
from typing import Dict, Optional


class SettlementState(str, Enum):
    Requested = "Requested"
    LockedAcked = "LockedAcked"
    Claimed = "Claimed"


class SettlementError(Exception):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def _sha256_hex(parts: list[str]) -> str:
    hasher = hashlib.sha256()
    for part in parts:
        hasher.update(part.encode("utf-8"))
    return hasher.hexdigest()


def compute_payment_hash(task_id: str, token_id: str) -> str:
    return _sha256_hex([task_id, token_id, "payment"])


def compute_proof_hash(task_id: str, payment_hash: str) -> str:
    return _sha256_hex([task_id, payment_hash, "proof"])


@dataclass
class SettlementRecord:
    task_id: str
    token_id: str
    payment_hash: str
    proof_hash: str
    btcpay_invoice_id: str
    btcpay_invoice_url: str
    state: SettlementState
    requested_at: int
    locked_at: Optional[int] = None
    claimed_at: Optional[int] = None
    last_error: Optional[str] = None

    def mark_locked(self, locked_at: int) -> None:
        if self.state == SettlementState.Requested:
            self.state = SettlementState.LockedAcked
            self.locked_at = locked_at

    def mark_claimed(self, proof_hash: str, claimed_at: int) -> None:
        if self.state != SettlementState.LockedAcked:
            raise SettlementError("proof_before_lock", "proof arrived before payment locked")
        if proof_hash != self.proof_hash:
            raise SettlementError("proof_hash_mismatch", "proof hash mismatch")
        self.state = SettlementState.Claimed
        self.claimed_at = claimed_at

    def to_dict(self) -> Dict[str, object]:
        data = asdict(self)
        data["state"] = self.state.value
        return data

    @classmethod
    def from_dict(cls, data: Dict[str, object]) -> "SettlementRecord":
        return cls(
            task_id=str(data["task_id"]),
            token_id=str(data["token_id"]),
            payment_hash=str(data["payment_hash"]),
            proof_hash=str(data["proof_hash"]),
            btcpay_invoice_id=str(data["btcpay_invoice_id"]),
            btcpay_invoice_url=str(data["btcpay_invoice_url"]),
            state=SettlementState(str(data["state"])),
            requested_at=int(data["requested_at"]),
            locked_at=int(data["locked_at"]) if data.get("locked_at") else None,
            claimed_at=int(data["claimed_at"]) if data.get("claimed_at") else None,
            last_error=str(data["last_error"]) if data.get("last_error") else None,
        )


class SettlementStore:
    def __init__(self, path: str) -> None:
        self.path = path
        self.records: Dict[str, SettlementRecord] = {}
        self._load()

    def _load(self) -> None:
        if not os.path.exists(self.path):
            return
        with open(self.path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
        records = payload.get("records", []) if isinstance(payload, dict) else payload
        for item in records or []:
            record = SettlementRecord.from_dict(item)
            self.records[record.task_id] = record

    def save(self) -> None:
        parent = os.path.dirname(self.path)
        if parent:
            os.makedirs(parent, exist_ok=True)
        payload = {"records": [record.to_dict() for record in self.records.values()]}
        with open(self.path, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, sort_keys=True)

    def upsert(self, record: SettlementRecord) -> None:
        self.records[record.task_id] = record
        self.save()

    def get(self, task_id: str) -> Optional[SettlementRecord]:
        return self.records.get(task_id)

    def get_by_invoice_id(self, invoice_id: str) -> Optional[SettlementRecord]:
        for record in self.records.values():
            if record.btcpay_invoice_id == invoice_id:
                return record
        return None
