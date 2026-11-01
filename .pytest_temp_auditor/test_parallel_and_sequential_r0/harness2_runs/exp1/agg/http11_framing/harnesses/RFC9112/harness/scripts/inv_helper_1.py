def synthesize_invariant_probes(host: str) -> list[bytes]:
    return [
        f'GET / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 5\r\n\r\n0\r\n\r\n'.encode('latin-1'),
        f'POST / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 4\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n'.encode('latin-1'),
        f'GET / HTTP/1.1\r\nHost: {host}\r\nX-Fold: val\r\n continued\r\n\r\n'.encode('latin-1'),
    ]
