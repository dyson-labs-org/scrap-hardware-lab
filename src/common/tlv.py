from dataclasses import dataclass
from typing import List, Optional, Tuple


class TLVError(ValueError):
    pass


@dataclass
class TLVRecord:
    tlv_type: int
    value: bytes


@dataclass
class TLVParseResult:
    records: List[TLVRecord]
    raw_without_signature: bytes
    signature: Optional[bytes]


def read_bigsize(data: bytes, offset: int) -> Tuple[int, int]:
    if offset >= len(data):
        raise TLVError("unexpected end of data")

    first = data[offset]
    if first < 0xFD:
        return first, offset + 1
    if first == 0xFD:
        if offset + 3 > len(data):
            raise TLVError("truncated bigsize (0xFD)")
        return int.from_bytes(data[offset + 1 : offset + 3], "big"), offset + 3
    if first == 0xFE:
        if offset + 5 > len(data):
            raise TLVError("truncated bigsize (0xFE)")
        return int.from_bytes(data[offset + 1 : offset + 5], "big"), offset + 5
    if offset + 9 > len(data):
        raise TLVError("truncated bigsize (0xFF)")
    return int.from_bytes(data[offset + 1 : offset + 9], "big"), offset + 9


def encode_bigsize(value: int) -> bytes:
    if value < 0 or value > 0xFFFFFFFFFFFFFFFF:
        raise TLVError("bigsize out of range")
    if value < 0xFD:
        return bytes([value])
    if value <= 0xFFFF:
        return b"\xFD" + value.to_bytes(2, "big")
    if value <= 0xFFFFFFFF:
        return b"\xFE" + value.to_bytes(4, "big")
    return b"\xFF" + value.to_bytes(8, "big")


def parse_tlv(data: bytes) -> TLVParseResult:
    records: List[TLVRecord] = []
    offset = 0
    last_type = -1
    signature: Optional[bytes] = None
    raw_without_signature = data

    while offset < len(data):
        record_start = offset
        tlv_type, offset = read_bigsize(data, offset)
        length, offset = read_bigsize(data, offset)
        if offset + length > len(data):
            raise TLVError("tlv length exceeds buffer")
        value = data[offset : offset + length]
        offset += length

        if tlv_type < last_type:
            raise TLVError("tlv records not in ascending type order")
        last_type = tlv_type

        if tlv_type == 240:
            # Signature must be last.
            if offset != len(data):
                raise TLVError("signature record is not last")
            signature = value
            raw_without_signature = data[:record_start]
            records.append(TLVRecord(tlv_type=tlv_type, value=value))
            break

        records.append(TLVRecord(tlv_type=tlv_type, value=value))

    return TLVParseResult(records=records, raw_without_signature=raw_without_signature, signature=signature)


def get_records(records: List[TLVRecord], tlv_type: int) -> List[bytes]:
    return [r.value for r in records if r.tlv_type == tlv_type]


def get_record(records: List[TLVRecord], tlv_type: int) -> Optional[bytes]:
    values = get_records(records, tlv_type)
    if not values:
        return None
    return values[0]
