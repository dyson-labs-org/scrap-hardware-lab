import json
import socket
from typing import Any, Dict, Tuple


def bind_socket(host: str, port: int) -> socket.socket:
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((host, port))
    return sock


def send_message(sock: socket.socket, host: str, port: int, message: Dict[str, Any]) -> None:
    data = json.dumps(message, sort_keys=True).encode("utf-8")
    sock.sendto(data, (host, port))


def recv_message(sock: socket.socket, timeout: float = 5.0) -> Tuple[Dict[str, Any], Tuple[str, int]]:
    sock.settimeout(timeout)
    data, addr = sock.recvfrom(65535)
    message = json.loads(data.decode("utf-8"))
    return message, addr
