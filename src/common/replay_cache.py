import json
import os
from dataclasses import dataclass
from typing import Dict


@dataclass
class ReplayCache:
    path: str

    def _load(self) -> Dict[str, int]:
        if not os.path.exists(self.path):
            return {}
        try:
            with open(self.path, "r", encoding="utf-8") as handle:
                data = json.load(handle)
            if isinstance(data, dict):
                return {k: int(v) for k, v in data.items()}
        except Exception:
            return {}
        return {}

    def _save(self, data: Dict[str, int]) -> None:
        os.makedirs(os.path.dirname(self.path), exist_ok=True)
        with open(self.path, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2, sort_keys=True)

    def check_and_add(self, token_id: bytes, expires_at: int, now: int) -> bool:
        cache = self._load()
        # Purge expired entries
        cache = {k: v for k, v in cache.items() if v >= now}

        key = token_id.hex()
        if key in cache:
            return False

        cache[key] = int(expires_at)
        self._save(cache)
        return True
