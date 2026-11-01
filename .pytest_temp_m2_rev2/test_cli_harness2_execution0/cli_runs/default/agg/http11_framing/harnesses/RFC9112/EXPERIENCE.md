# Experience Record: RFC9112

- **Domain**: `http11_framing`
- **Adopted Candidate**: `agg1`
- **Behavioral Contrast Score**: `42.50`
- **Cue**: When auditing RFC9112 in domain `http11_framing` for boundary desynchronization
- **Lesson**: Combine invariant byte-length ruler checks with http11_framing specialist skills (desync_framing, dom_http11_framing_1, inv_ruler_norm_1) to surface parser divergences without false positives.
- **Evidence**: Composed harness `agg1` improved behavioral score by +20.5 over base (Divergences=3, SimVulns=2).
