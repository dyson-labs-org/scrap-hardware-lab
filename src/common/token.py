from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from .crypto import TAG_TOKEN, tagged_hash, load_schnorr_engine
from .replay_cache import ReplayCache
from .tlv import TLVError, get_record, get_records, parse_tlv

TLV_VERSION = 0
TLV_ISSUER = 2
TLV_SUBJECT = 4
TLV_AUDIENCE = 6
TLV_ISSUED_AT = 8
TLV_EXPIRES_AT = 10
TLV_TOKEN_ID = 12
TLV_CAPABILITY = 14
TLV_SIGNATURE = 240

TLV_CONSTRAINT_GEO = 13
TLV_CONSTRAINT_RATE = 15
TLV_CONSTRAINT_AMOUNT = 17
TLV_CONSTRAINT_AFTER = 19

TLV_DELEGATION_TYPES = {20, 22, 24, 26}

KNOWN_EVEN_TYPES = {
    TLV_VERSION,
    TLV_ISSUER,
    TLV_SUBJECT,
    TLV_AUDIENCE,
    TLV_ISSUED_AT,
    TLV_EXPIRES_AT,
    TLV_TOKEN_ID,
    TLV_CAPABILITY,
    TLV_SIGNATURE,
    TLV_CONSTRAINT_RATE,
    TLV_CONSTRAINT_AMOUNT,
    TLV_CONSTRAINT_AFTER,
    *TLV_DELEGATION_TYPES,
}

KNOWN_ODD_TYPES = {TLV_CONSTRAINT_GEO}


@dataclass
class CapabilityToken:
    version: int
    issuer: bytes
    subject: str
    audience: str
    issued_at: int
    expires_at: int
    token_id: bytes
    capabilities: List[str]
    signature: Optional[bytes]
    raw_without_signature: bytes
    constraints: Dict[str, bytes] = field(default_factory=dict)
    delegation: Dict[str, bytes] = field(default_factory=dict)

    @staticmethod
    def _decode_utf8(value: bytes) -> str:
        try:
            return value.decode("utf-8")
        except Exception:
            return value.hex()

    @classmethod
    def from_bytes(cls, data: bytes) -> "CapabilityToken":
        parsed = parse_tlv(data)
        records = parsed.records

        unknown_even = []
        for record in records:
            if record.tlv_type % 2 == 0 and record.tlv_type not in KNOWN_EVEN_TYPES:
                unknown_even.append(record.tlv_type)
            if record.tlv_type % 2 == 1 and record.tlv_type not in KNOWN_ODD_TYPES:
                # Unknown odd types are ignored per spec.
                continue

        if unknown_even:
            raise TLVError(f"unknown even TLV types: {unknown_even}")

        version_raw = get_record(records, TLV_VERSION)
        issuer = get_record(records, TLV_ISSUER)
        subject = get_record(records, TLV_SUBJECT)
        audience = get_record(records, TLV_AUDIENCE)
        issued_at = get_record(records, TLV_ISSUED_AT)
        expires_at = get_record(records, TLV_EXPIRES_AT)
        token_id = get_record(records, TLV_TOKEN_ID)
        capabilities = [cls._decode_utf8(c) for c in get_records(records, TLV_CAPABILITY)]

        if version_raw is None or issuer is None or subject is None or audience is None:
            raise TLVError("missing required fields")
        if issued_at is None or expires_at is None or token_id is None:
            raise TLVError("missing required timing/token fields")
        if not capabilities:
            raise TLVError("no capabilities present")

        version = int.from_bytes(version_raw, "big")
        issued_at_i = int.from_bytes(issued_at, "big")
        expires_at_i = int.from_bytes(expires_at, "big")

        constraints = {}
        for tlv_type, name in [
            (TLV_CONSTRAINT_GEO, "constraint_geo"),
            (TLV_CONSTRAINT_RATE, "constraint_rate"),
            (TLV_CONSTRAINT_AMOUNT, "constraint_amount"),
            (TLV_CONSTRAINT_AFTER, "constraint_after"),
        ]:
            value = get_record(records, tlv_type)
            if value is not None:
                constraints[name] = value

        delegation = {}
        for tlv_type, name in [
            (20, "root_issuer"),
            (22, "root_token_id"),
            (24, "parent_token_id"),
            (26, "chain_depth"),
        ]:
            value = get_record(records, tlv_type)
            if value is not None:
                delegation[name] = value

        return cls(
            version=version,
            issuer=issuer,
            subject=cls._decode_utf8(subject),
            audience=cls._decode_utf8(audience),
            issued_at=issued_at_i,
            expires_at=expires_at_i,
            token_id=token_id,
            capabilities=capabilities,
            signature=parsed.signature,
            raw_without_signature=parsed.raw_without_signature,
            constraints=constraints,
            delegation=delegation,
        )

    def verify(
        self,
        now: int,
        expected_audience: str,
        required_capability: Optional[str],
        operator_pubkey: bytes,
        replay_cache: Optional[ReplayCache],
        allow_mock_signatures: bool,
    ) -> Tuple[bool, List[str], List[str]]:
        issues: List[str] = []
        notes: List[str] = []

        if self.audience != expected_audience:
            issues.append(f"audience mismatch (token={self.audience} expected={expected_audience})")

        if now < self.issued_at:
            issues.append("token not yet valid")
        if now > self.expires_at:
            issues.append("token expired")

        if required_capability and required_capability not in self.capabilities:
            issues.append("capability not granted by token")

        constraint_after = self.constraints.get("constraint_after")
        if constraint_after is not None:
            not_before = int.from_bytes(constraint_after, "big")
            if now < not_before:
                issues.append("constraint_after not satisfied")

        if any(k for k in self.constraints if k not in {"constraint_after"}):
            notes.append("constraints present but not enforced in demo")

        if self.signature is None:
            issues.append("missing token signature")
        else:
            engine = load_schnorr_engine()
            msg32 = tagged_hash(TAG_TOKEN, self.raw_without_signature)
            verified = engine.verify(msg32, self.signature, operator_pubkey)
            if verified is None:
                if allow_mock_signatures:
                    notes.append("signature verification skipped (mock mode)")
                else:
                    issues.append("signature verification unavailable (install coincurve or allow mock)")
            elif not verified:
                issues.append("token signature invalid")

        # Replay check must occur after stateless validation.
        if replay_cache is not None and not issues:
            if not replay_cache.check_and_add(self.token_id, self.expires_at, now):
                issues.append("replay detected (token_id already used)")

        return len(issues) == 0, issues, notes

    def to_dict(self) -> Dict[str, str]:
        return {
            "version": str(self.version),
            "issuer": self.issuer.hex(),
            "subject": self.subject,
            "audience": self.audience,
            "issued_at": str(self.issued_at),
            "expires_at": str(self.expires_at),
            "token_id": self.token_id.hex(),
            "capabilities": ",".join(self.capabilities),
        }
