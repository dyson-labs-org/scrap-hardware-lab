import hashlib
from dataclasses import dataclass
from typing import Optional


TAG_TOKEN = "SCRAP/token/v1"
TAG_BINDING = "SCRAP/binding/v1"
TAG_PROOF = "SCRAP/proof/v1"
TAG_TASK = "SCRAP/task/v1"


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def tagged_hash(tag: str, msg: bytes) -> bytes:
    tag_hash = sha256(tag.encode("utf-8"))
    return sha256(tag_hash + tag_hash + msg)


@dataclass
class SchnorrEngine:
    name: str
    available: bool

    def sign(self, msg32: bytes, privkey: bytes) -> Optional[bytes]:
        if not self.available:
            return None
        raise NotImplementedError

    def verify(self, msg32: bytes, sig64: bytes, pubkey: bytes) -> Optional[bool]:
        if not self.available:
            return None
        raise NotImplementedError


def load_schnorr_engine() -> SchnorrEngine:
    # Mock-only: no external crypto dependencies in the lab demo.
    return SchnorrEngine(name="mock", available=False)
