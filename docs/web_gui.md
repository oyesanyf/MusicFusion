# ACDSO Web GUI &amp; CLI Reference

This is the field-by-field reference for the web GUI (`start.py`) and the
`run_acdso.py` CLI it wraps. For a quick start, see the
[Web GUI section of the README](../README.md#-web-gui).

Two ground rules that shape everything below:

1. **The GUI has no logic of its own.** Every control maps to one
   `run_acdso.py` flag. Picking options builds a command line (shown live in
   the "Command" panel) and runs it as a subprocess. If a flag does nothing
   on the CLI, it does nothing from the GUI either — that's called out
   explicitly per-flag below rather than hidden.
2. **"Predict" and "Train-only" are the only two things you actually choose.**
   Nearly every run either trains-and-predicts (`--predict`) or
   trains-only (`--target`). The rest of the flags tune *how* that training
   happens, not *whether* it happens.

---

## How a run is actually dispatched

`run_acdso.py`'s `main()` decides what to do in this order:

1. If `--target` has **multiple comma-separated columns**, it trains one
   model per target (or one multi-output model with `--multi-output`) and
   returns. This path ignores everything below.
2. If `--predict` is set, it trains on `--target` (or on `--predict` itself
   if `--target` is empty), then makes predictions and writes
   `predictions_<file>_<target>_full_dataset.csv`. This is the main path.
3. Otherwise, **`--autonomous` runs unconditionally** — it defaults to
   `True` and nothing in the script ever sets it back to `False`. Because of
   that, the flags described as "dead" in the tables below
   (`--v3`, `--policy`, `--decision`, `--timeseries`, `--all`, and the
   benchmark-only-without-a-file path that reads `--datasets`/`--output`)
   can never run — the autonomous branch always wins first. This is a
   pre-existing property of `run_acdso.py`, not something the GUI does; the
   GUI simply surfaces it honestly instead of hiding it.

If you always upload a file and pick a predict/target column (the normal
GUI flow), you are always on path 2.

---

## CLI flags (`run_acdso.py`)

### File &amp; target selection

| Flag | Type | Default | What it does |
|---|---|---|---|
| `--file` / `-f` | path | none | The data file to load: CSV, Excel (`.xlsx`/`.xls`), Parquet, JSON, Feather, or Pickle. Without it, ACDSO runs its bundled demo datasets instead of your data. |
| `--predict` / `-p` | column name(s) | none | **The main path.** Trains a fresh model (never loads a cached one) and writes predictions for every row of `--file`. Comma-separate for multiple targets — each gets its own model, run sequentially. |
| `--target` / `-t` | column name(s) | none | Trains without predicting. Comma-separate for multi-target *training* (see dispatch step 1 above). Redundant to set alongside `--predict` for a single column — `--predict`'s training uses `--target` as the actual training column if both are given and differ, which triggers a mismatch warning (see below). |
| `--multi-output` | flag | off | Only meaningful with a comma-separated `--target`: trains **one** model that predicts all targets at once, instead of one model per target. |
| `--task` | `auto` \| `classification` \| `regression` | `auto` | Forces the task type instead of letting ACDSO infer it from the target column. |
| `--exclude` / `-e` | column name(s) | none | Columns to drop before training, on top of whatever `--no-auto-exclude` doesn't disable. |
| `--no-auto-exclude` | flag | off | Turns off ACDSO's automatic leakage/ID/constant-column detection (rule-based, or LLM-powered if `OPENAI_API_KEY` is set). |

**`--target` vs `--predict` mismatch:** if both are set to *different* single
columns, training uses `--target`, but the tool still tries to predict
whatever `--predict` says. It prints a warning and falls back to predicting
the trained column instead. The GUI's live column validator (see below) now
catches this case before the run starts, rather than after a multi-minute
training run.

### Pipeline phases

| Flag | Type | Default | What it does |
|---|---|---|---|
| `--autonomous` / `-a` | flag | **always True** | Runs EDA → Risk → AutoML → Time Series. There is no way to turn this off — see "How a run is actually dispatched" above. |
| `--simple` / `--quick` | flag | off | Skips the benchmark phase (sets `--skip-benchmark`) for a faster run. Time Series still runs regardless — `--simple` does not skip it, despite what the name suggests. |
| `--benchmark` / `-b` | flag | off | With `--file`: runs the full pipeline *and* benchmarks ACDSO against FLAML/TPOT/H2O/AutoGluon/etc., producing one combined report. Slow — 10–30 minutes per target. Without `--file`: benchmarks on built-in datasets only (see "Dead in current dispatch" below). |
| `--skip-benchmark` | flag | off | Skips the benchmark comparison inside the `--predict` path. Recommended for multi-target runs. |
| `--force` / `--fresh` | flag | off | Deletes this dataset's existing models/reports/predictions before training, so nothing stale is reused. |
| `--v3` | flag | off | **Dead in current dispatch** (see above) — the `--autonomous` branch always wins first, so this never executes. |
| `--policy` | flag | off | **Dead in current dispatch**, same reason. |
| `--decision` / `-d` | flag | off | **Dead in current dispatch**, same reason. Also: even if reached, `run_autonomous_demo()` (the function this path calls into) has no parameter for it at all — `--treatment`/`--budget` have nowhere to go. |
| `--timeseries` / `-ts` | flag | off | **Dead as a standalone flag** — Time Series already runs unconditionally inside the autonomous pipeline. Toggling this does nothing either way. |
| `--all` | flag | off | **Dead in current dispatch** — intended to run the bundled demo datasets, but the autonomous branch always wins first. |

### Time series

| Flag | Type | Default | What it does |
|---|---|---|---|
| `--datetime-col` | column name | none | The date/period column, used by the Time Series phase that always runs inside `--autonomous`/`--predict`. This one is real and matters. |
| `--horizon` | int | `7` | How many periods ahead to forecast. Also real. |

### Decision intelligence (currently non-functional from the CLI's top-level dispatch)

| Flag | Type | Default | What it does |
|---|---|---|---|
| `--treatment` | column name | none | Would name the intervention column for causal/uplift analysis. **Not read by any reachable code path** — `run_autonomous_demo()` doesn't accept it, and `--decision`'s own branch is unreachable (see above). |
| `--budget` | float | none | Would cap an optimization's spend. Same caveat as `--treatment`. |

### Model selection &amp; runtime

| Flag | Type | Default | What it does |
|---|---|---|---|
| `--best-score` | flag | off | Selects the single highest-CV-score model, ignoring ACDSO's usual multi-objective trade-off (speed / memory / latency / risk / robustness). Can pick a slower or larger model if it scores higher. |
| `--time-budget` | int (seconds) | `21600` (6h) | Per-system time budget for the built-in-datasets benchmark path. **Only reachable when running `--benchmark` with no `--file`** — which is itself unreachable per the dispatch note above, so this currently has no effect from a normal GUI run. |
| `--datasets` | names or `all` | `tips` | Which built-in datasets to benchmark. Same reachability caveat as `--time-budget`. |
| `--output` / `-o` | path | none | Where to write benchmark results JSON. Same reachability caveat. |

---

## Web GUI (`start.py` + `src/acdso/webui/`)

### 1 · Data file

Drop or browse to a file (same formats as `--file`). On upload, the GUI
reads a preview (first 10 rows) and the full column list/dtypes, and
pre-fills the Forecast card's period/value columns by guessing (a
non-numeric, date-shaped column name for the period; the most
plausibly-named numeric column for the value). These are *guesses* — check
them.

### Target

- **Predict target(s)** → `--predict`. Multi-select; this is the field to
  use for almost everything.
- **Train-only target(s)** → `--target`. Leave empty when using Predict —
  setting both to different columns triggers the mismatch behavior
  described above.
- **Multi-output model** → `--multi-output`.
- **Task type** → `--task`.

### Columns

- **Exclude columns** → `--exclude`.
- **Disable auto-exclusion** → `--no-auto-exclude`.

### Pipeline

Maps directly to the Pipeline flags table above, including the ones marked
dead there (`v3`, `policy`, `decision`, `timeseries` mode, `all`). They're
still present in the GUI rather than removed, so the command preview stays
an honest 1:1 mirror of what `run_acdso.py` accepts — but toggling them will
not change what actually runs.

### Time series

- **Datetime column** → `--datetime-col`.
- **Forecast horizon** → `--horizon`.

### Decision

- **Treatment column** → `--treatment`, **Budget constraint** → `--budget`.
  Both currently non-functional from this GUI's run path — see the CLI
  table above.

### Runtime

- **Select by best CV score** → `--best-score`.
- **Time budget (seconds)**, **Benchmark datasets**, **Benchmark output
  JSON** → `--time-budget` / `--datasets` / `--output`. Currently
  non-functional from a file-based run — see the CLI table above.
  `--datasets` intentionally has **no default value** in the GUI (it used to
  default to `"tips"`, which looked like it was reading something from your
  uploaded data; it wasn't).
- **Fresh run (--force)** → `--force`.

### Column validation (live, before you click Run)

As you change Predict/Train-only/Exclude/Datetime-column selections, the
GUI calls `/api/validate-columns` (debounced ~500ms) and shows the result
above the Run button:

- **Rule-based check (always runs):** a column can't be assigned to two
  roles at once — e.g. the same column as both a training target and the
  datetime column. This is deterministic and **always blocks** the run,
  client-side and again server-side in `/api/run` as the authoritative gate
  (so a stale page or a direct API call can't bypass it).
- **LLM check (only if `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or a local
  Ollama model is available):** adds softer judgment on top — e.g. flagging
  an ID-like column picked as a prediction target, or a target column that
  looks like it duplicates the datetime column. **LLM findings are always
  advisory (`warn`), never blocking on their own** — they can be wrong (an
  earlier version of this check briefly rejected a column pairing that was
  actually correct, by misreading a numeric column's string-formatted
  sample), so only the deterministic same-column-two-roles rule is allowed
  to stop a run outright.
- With no LLM provider configured, only the rule-based check runs — same
  blocking behavior, just without the softer warnings.

### Forecast (separate from the Run pipeline / AutoML path)

Aggregates the file to one value per period and projects it forward with a
short, transparent method (last-value, drift, or linear trend — whichever
backtests best), rather than fitting a full AutoML model at row grain. Use
this for "how much next month" questions; it exists because `AutoTS` has no
`forecast(horizon)` method and row-level AutoML can't extrapolate a trend at
all.

- **Period column**, **Value column** — auto-guessed on upload (see above),
  editable. The guess never picks a numeric column for the period slot.
- **Aggregate** — `sum` / `mean` / `max` / `count` per period.
- **Periods** — how many periods ahead to project.
- **Drop first / last period (partial)** — excludes the first/last period
  from the aggregation. Check these if your export's date range doesn't
  align to calendar period boundaries (e.g. a file starting mid-month), so
  a partial period doesn't skew the average.
- Output: a table of actual + forecast values (byte-valued columns shown in
  GB), the method chosen and its backtested error, a comparison of all
  candidate methods, and a **Download CSV** button that writes a uniquely
  time-stamped file to `reports/` (never overwrites a previous forecast
  download) with both the raw and GB-converted columns.

### Command / Live output / Artifacts / Report History

- **Command** mirrors exactly what will run (client-side JS rebuilds the
  same logic as the server's `build_argv()`), so what you see is what
  executes — no separate code path.
- **Live output** streams the subprocess's stdout, also saved to
  `reports/webui_logs/run_<id>.log`.
- **Artifacts** lists only files touched by *this* run.
- **Report History** lists *everything* ever written to `reports/` — past
  sessions included — with a filename filter and view/download links.
  Refreshes automatically when a run finishes.

### Serving

`start.py` runs under [waitress](https://docs.pylonsproject.org/projects/waitress/),
a production-grade pure-Python WSGI server (Windows-friendly, unlike
gunicorn). Flask's own dev server is used only with `--debug`, for editing
the GUI's own code with the auto-reloader. If waitress isn't installed,
`start.py` falls back to Flask's dev server with a warning rather than
failing outright.

Only one run at a time: ACDSO writes artifacts under deterministic,
dataset-derived names, so a second concurrent run would silently overwrite
the first one's output. `/api/run` returns 409 if a run is already in
progress.
