def build_cl0_probe(host: str = '127.0.0.1') -> bytes:
    return f'GET / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 5\r\n\r\n0\r\n\r\n'.encode('latin-1')
