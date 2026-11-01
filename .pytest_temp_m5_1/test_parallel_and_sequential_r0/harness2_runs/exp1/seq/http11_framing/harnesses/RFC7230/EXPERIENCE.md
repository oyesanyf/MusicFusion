# Experience Record: RFC7230

- **Domain**: `http11_framing`
- **Adopted Candidate**: `c2`
- **Behavioral Contrast Score**: `49.00`
- **Cue**: When sequentially refining harnesses across 2 depth rounds on RFC7230 (`http11_framing`)
- **Lesson**: Carry composed c1..c8 trees across sequential rounds to accumulate specialist skills (desync_framing, dom_http11_framing_1, dom_http11_framing_2) and eliminate framing false positives.
- **Evidence**: Sequential composition `c2` achieved score 49.0 (+27.0 over base) with R=8 rollouts and P=6 stages.
