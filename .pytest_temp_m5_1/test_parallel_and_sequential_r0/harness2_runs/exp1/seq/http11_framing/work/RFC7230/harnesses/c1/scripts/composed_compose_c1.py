def composed_probe_suite(host: str = '127.0.0.1') -> list[bytes]:
    return [
        f'GET / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 5\r\n\r\n0\r\n\r\n'.encode('latin-1'),
        f'HEAD / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 5\r\n\r\n0\r\n\r\n'.encode('latin-1'),
        f'POST / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 4\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n'.encode('latin-1'),
        f'POST /nul HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\n\r\n'.encode('latin-1'),
        f'GET / HTTP/1.1\r\nHost: {host}\r\nX-Foo:\tbar\r\n\r\n'.encode('latin-1'),
    ]
