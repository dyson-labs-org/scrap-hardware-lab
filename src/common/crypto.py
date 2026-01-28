import hashlib
import os
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


def xonly_pubkey(pubkey: bytes) -> bytes:
    if len(pubkey) == 32:
        return pubkey
    if len(pubkey) == 33 and pubkey[0] in (2, 3):
        return pubkey[1:]
    raise ValueError("unexpected public key length")


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


class CoincurveSchnorr(SchnorrEngine):
    def __init__(self, module):
        super().__init__(name="coincurve", available=True)
        self._module = module

    def sign(self, msg32: bytes, privkey: bytes) -> Optional[bytes]:
        return self._module.schnorr.sign(msg32, privkey, aux_rand=os.urandom(32))

    def verify(self, msg32: bytes, sig64: bytes, pubkey: bytes) -> Optional[bool]:
        pubkey_x = xonly_pubkey(pubkey)
        return bool(self._module.schnorr.verify(sig64, msg32, pubkey_x))


def load_schnorr_engine() -> SchnorrEngine:
    try:
        import coincurve  # type: ignore

        return CoincurveSchnorr(coincurve)
    except Exception:
        return SchnorrEngine(name="unavailable", available=False)
