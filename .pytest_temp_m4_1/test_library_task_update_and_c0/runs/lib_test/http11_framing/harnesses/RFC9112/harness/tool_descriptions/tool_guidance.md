# Tool Execution Guidance (c4)
1. Execute 3-Step Canary Pipeline: REQ 1 (Baseline) -> REQ 2 (Candidate) -> REQ 3 (Canary).
2. Calibrate front-end header buffer limits via Protocol Ruler binary search.
3. Delta-debug confirmed anomalies down to the minimal triggering byte sequence.
