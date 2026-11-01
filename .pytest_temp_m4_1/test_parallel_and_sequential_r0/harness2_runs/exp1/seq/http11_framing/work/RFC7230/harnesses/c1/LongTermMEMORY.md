# Long-Term Empirical Protocol Memory (c3) - Domain: http11_framing
- Front-end terminators frequently normalize obsolete line folding (obs-fold) into SP.
- GET/HEAD requests carrying non-zero Content-Length (CL.0) trigger backend queue poisoning.
- [DOM-1 / RFC7230] Exploit http11_framing state-machine transition ambiguities between front-end and origin.
