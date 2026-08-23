"""Flask application backing the ACDSO web GUI.

Design notes
------------
The pipeline itself lives in ``run_acdso.py`` (~11k lines, argparse driven).
Rather than re-import and re-wire its internals, this GUI builds the exact
same command line and runs it as a subprocess, streaming stdout back to the
browser.  That keeps the GUI honest: every flag exposed here means the same
thing it means on the command line, and there is no second code path to keep
in sync.

One run at a time.  ACDSO writes artifacts to ``reports/`` under
deterministic, dataset-derived names (``reports/predictions_<base>_<target>
_full_dataset.csv`` and friends), so two concurrent runs on the same file
would silently clobber each other's output.  The manager refuses to start a
second job while one is live.
"""

from __future__ import annotations

import csv
import json
import os
import re
import shlex
import subprocess
import sys
import threading
import time
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any

from flask import Flask, abort, jsonify, render_template, request, send_file
from werkzeug.utils import secure_filename

from .column_validator import validate_columns
from .forecast import SeriesSpec, run_forecast

# src/acdso/webui/app.py -> src/acdso/webui -> src/acdso -> src -> <repo root>
REPO_ROOT = Path(__file__).resolve().parents[3]
RUNNER = REPO_ROOT / "run_acdso.py"
UPLOAD_DIR = REPO_ROOT / "uploads"
REPORTS_DIR = REPO_ROOT / "reports"
LOG_DIR = REPORTS_DIR / "webui_logs"

#: Formats ``run_acdso.py`` documents as loadable.
ALLOWED_SUFFIXES = {".csv", ".xlsx", ".xls", ".parquet", ".json", ".feather", ".pkl"}

MAX_UPLOAD_BYTES = 512 * 1024 * 1024
MAX_LOG_LINES = 40_000


# ---------------------------------------------------------------------------
# Flag specification -- single source of truth for the form and for argv
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Flag:
    """One CLI flag, rendered into the form and back out into argv."""

    name: str  # form field name
    cli: str  # flag as passed to run_acdso.py
    kind: str  # bool | text | int | float | choice | columns
    label: str
    help: str = ""
    default: Any = None
    choices: tuple[str, ...] = ()
    group: str = "General"


FLAGS: tuple[Flag, ...] = (
    # --- What to predict -------------------------------------------------
    Flag(
        "predict", "--predict", "columns", "Predict target(s)",
        "Train fresh model(s) and write predictions for the whole dataset. "
        "This is the main path: pick one or more columns.",
        group="Target",
    ),
    Flag(
        "target", "--target", "columns", "Train-only target(s)",
        "Train without the prediction step. Leave empty when using "
        "'Predict target(s)' above -- setting both is redundant.",
        group="Target",
    ),
    Flag(
        "multi_output", "--multi-output", "bool", "Multi-output model",
        "One model predicting all targets at once instead of one model per target.",
        default=False, group="Target",
    ),
    Flag(
        "task", "--task", "choice", "Task type",
        "Leave on 'auto' to let ACDSO infer classification vs regression.",
        default="auto", choices=("auto", "classification", "regression"),
        group="Target",
    ),
    # --- Columns ---------------------------------------------------------
    Flag(
        "exclude", "--exclude", "columns", "Exclude columns",
        "Columns to drop before training.",
        group="Columns",
    ),
    Flag(
        "no_auto_exclude", "--no-auto-exclude", "bool", "Disable auto-exclusion",
        "Turn off automatic leakage / ID / constant-column detection.",
        default=False, group="Columns",
    ),
    # --- Pipeline phases -------------------------------------------------
    Flag(
        "autonomous", "--autonomous", "bool", "Autonomous pipeline",
        "Full pipeline: EDA -> Risk -> AutoML -> Time Series. "
        "On by default in the CLI whenever a file is given.",
        default=True, group="Pipeline",
    ),
    Flag(
        "simple", "--simple", "bool", "Quick mode",
        "EDA + AutoML only. Skips Time Series and benchmarks.",
        default=False, group="Pipeline",
    ),
    Flag(
        "benchmark", "--benchmark", "bool", "Run benchmarks",
        "Compare against FLAML, TPOT, H2O, AutoGluon. Slow -- 10-30 min per target.",
        default=False, group="Pipeline",
    ),
    Flag(
        "skip_benchmark", "--skip-benchmark", "bool", "Skip benchmark phase",
        "Skip benchmarking inside autonomous mode. Recommended for multi-target runs.",
        default=True, group="Pipeline",
    ),
    Flag(
        "timeseries", "--timeseries", "bool", "Time series mode",
        "Run the Auto Time Series path.",
        default=False, group="Pipeline",
    ),
    Flag(
        "decision", "--decision", "bool", "Decision intelligence",
        "Causal analysis, uplift modelling and action optimization.",
        default=False, group="Pipeline",
    ),
    Flag(
        "policy", "--policy", "bool", "Policy learning (RLlib)",
        "Reinforcement-learning policy optimization demo.",
        default=False, group="Pipeline",
    ),
    Flag(
        "v3", "--v3", "bool", "v3 innovations",
        "Stress testing, successive halving, risk calibration.",
        default=False, group="Pipeline",
    ),
    Flag(
        "all", "--all", "bool", "Run all built-in demos",
        "Ignores the uploaded file and runs the bundled demo datasets.",
        default=False, group="Pipeline",
    ),
    # --- Time series -----------------------------------------------------
    Flag(
        "datetime_col", "--datetime-col", "columns", "Datetime column",
        "Timestamp column for time-series work.",
        group="Time series",
    ),
    Flag(
        "horizon", "--horizon", "int", "Forecast horizon",
        "Steps to forecast ahead.",
        default=7, group="Time series",
    ),
    # --- Decision intelligence -------------------------------------------
    Flag(
        "treatment", "--treatment", "columns", "Treatment column",
        "The intervention column, e.g. 'offered_incentive'.",
        group="Decision",
    ),
    Flag(
        "budget", "--budget", "float", "Budget constraint",
        "Budget ceiling for action optimization.",
        group="Decision",
    ),
    # --- Model selection / runtime ---------------------------------------
    Flag(
        "best_score", "--best-score", "bool", "Select by best CV score",
        "Pick the highest-scoring model instead of the multi-objective "
        "knee-point trade-off (speed / memory / risk).",
        default=False, group="Runtime",
    ),
    Flag(
        "time_budget", "--time-budget", "int", "Time budget (seconds)",
        "Per-system budget used during benchmarking. CLI default is 21600 (6 h).",
        default=21600, group="Runtime",
    ),
    Flag(
        "force", "--force", "bool", "Fresh run (--force)",
        "Delete existing models and reports for this dataset before training.",
        default=False, group="Runtime",
    ),
    Flag(
        "datasets", "--datasets", "text", "Benchmark datasets",
        "Comma-separated built-in dataset names, or 'all'. Only used for "
        "benchmark runs without an uploaded file. Leave blank to omit the "
        "flag entirely (run_acdso.py falls back to its own default, 'tips').",
        default=None, group="Runtime",
    ),
    Flag(
        "output", "--output", "text", "Benchmark output JSON",
        "Optional path for benchmark results JSON.",
        group="Runtime",
    ),
)

FLAGS_BY_NAME = {f.name: f for f in FLAGS}


# ---------------------------------------------------------------------------
# Uploads
# ---------------------------------------------------------------------------


@dataclass
class Upload:
    id: str
    path: Path
    original_name: str
    columns: list[str]
    rows: int | None
    preview: list[dict[str, Any]]


def _read_frame(path: Path, nrows: int | None = 50):
    """Load an uploaded file. ``nrows=None`` reads all of it.

    A slice is enough for column discovery and the preview; forecasting
    needs every row, since the whole point is aggregating them.
    """
    import pandas as pd

    def limit(frame):
        return frame if nrows is None else frame.head(nrows)

    suffix = path.suffix.lower()
    if suffix == ".csv":
        return pd.read_csv(path, nrows=nrows)
    if suffix in {".xlsx", ".xls"}:
        return pd.read_excel(path, nrows=nrows)
    if suffix == ".parquet":
        return limit(pd.read_parquet(path))
    if suffix == ".json":
        return limit(pd.read_json(path))
    if suffix == ".feather":
        return limit(pd.read_feather(path))
    if suffix == ".pkl":
        return limit(pd.read_pickle(path))
    raise ValueError(f"Unsupported file type: {suffix}")


def _json_safe_preview(frame, limit: int = 10) -> list[dict[str, Any]]:
    """First rows as JSON-serialisable dicts, NaN collapsed to null."""
    head = frame.head(limit)
    return head.astype(object).where(head.notna(), None).to_dict("records")


def _row_count(path: Path) -> int | None:
    """Cheap row count for CSV; ``None`` where counting is not cheap."""
    if path.suffix.lower() != ".csv":
        return None
    try:
        with path.open("r", encoding="utf-8", errors="replace") as fh:
            return max(sum(1 for _ in fh) - 1, 0)
    except OSError:
        return None


# ---------------------------------------------------------------------------
# Job execution
# ---------------------------------------------------------------------------


@dataclass
class Job:
    id: str
    argv: list[str]
    started_at: float
    log_path: Path
    lines: list[str] = field(default_factory=list)
    status: str = "running"  # running | finished | failed | cancelled
    returncode: int | None = None
    dropped_lines: int = 0
    proc: subprocess.Popen | None = None
    _lock: threading.Lock = field(default_factory=threading.Lock)

    @property
    def command(self) -> str:
        return " ".join(shlex.quote(a) for a in self.argv)

    def append(self, line: str) -> None:
        with self._lock:
            self.lines.append(line)
            if len(self.lines) > MAX_LOG_LINES:
                overflow = len(self.lines) - MAX_LOG_LINES
                del self.lines[:overflow]
                self.dropped_lines += overflow

    def slice_from(self, cursor: int) -> tuple[list[str], int]:
        """Log lines after ``cursor``, in absolute line numbering."""
        with self._lock:
            base = self.dropped_lines
            start = max(cursor - base, 0)
            return list(self.lines[start:]), base + len(self.lines)


class JobManager:
    """Runs one ACDSO job at a time, keeping its log in memory and on disk."""

    def __init__(self) -> None:
        self._jobs: dict[str, Job] = {}
        self._lock = threading.Lock()
        self._active: str | None = None

    @property
    def active_job(self) -> Job | None:
        with self._lock:
            return self._jobs.get(self._active) if self._active else None

    def get(self, job_id: str) -> Job | None:
        with self._lock:
            return self._jobs.get(job_id)

    def recent(self, limit: int = 20) -> list[Job]:
        with self._lock:
            jobs = sorted(self._jobs.values(), key=lambda j: j.started_at, reverse=True)
        return jobs[:limit]

    def start(self, argv: list[str]) -> Job:
        with self._lock:
            active = self._jobs.get(self._active) if self._active else None
            if active is not None and active.status == "running":
                raise RuntimeError(
                    "A run is already in progress. Stop it or wait for it to finish "
                    "-- concurrent runs overwrite each other's reports."
                )
            job_id = uuid.uuid4().hex[:12]
            LOG_DIR.mkdir(parents=True, exist_ok=True)
            job = Job(
                id=job_id,
                argv=argv,
                started_at=time.time(),
                log_path=LOG_DIR / f"run_{job_id}.log",
            )
            self._jobs[job_id] = job
            self._active = job_id

        env = dict(os.environ)
        env["PYTHONUNBUFFERED"] = "1"
        # run_acdso.py prints arrows and box-drawing characters; without this
        # the child dies on Windows' default cp1252 stdout encoding.
        env["PYTHONIOENCODING"] = "utf-8"

        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0

        job.proc = subprocess.Popen(
            argv,
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            stdin=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            env=env,
            creationflags=creationflags,
        )
        job.append(f"$ {job.command}")
        threading.Thread(target=self._pump, args=(job,), daemon=True).start()
        return job

    def _pump(self, job: Job) -> None:
        """Drain the child's stdout into the job log until it exits."""
        proc = job.proc
        assert proc is not None and proc.stdout is not None
        try:
            with job.log_path.open("w", encoding="utf-8", errors="replace") as log:
                log.write(f"$ {job.command}\n")
                for raw in proc.stdout:
                    line = raw.rstrip("\n")
                    job.append(line)
                    log.write(line + "\n")
                    log.flush()
        finally:
            job.returncode = proc.wait()
            if job.status != "cancelled":
                job.status = "finished" if job.returncode == 0 else "failed"
            job.append(
                f"[webui] process exited with code {job.returncode} "
                f"after {time.time() - job.started_at:.1f}s"
            )

    def stop(self, job: Job) -> bool:
        if job.proc is None or job.proc.poll() is not None:
            return False
        job.status = "cancelled"
        if sys.platform == "win32":
            # ACDSO spawns worker processes (LightGBM / H2O / Ray); kill the tree.
            subprocess.run(
                ["taskkill", "/F", "/T", "/PID", str(job.proc.pid)],
                capture_output=True,
                check=False,
            )
        else:
            job.proc.terminate()
        return True


# ---------------------------------------------------------------------------
# argv construction
# ---------------------------------------------------------------------------


def _as_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [p.strip() for p in value.split(",") if p.strip()]
    return [str(v).strip() for v in value if str(v).strip()]


def build_argv(payload: dict[str, Any], data_file: Path | None) -> list[str]:
    """Translate a form payload into a ``run_acdso.py`` command line."""
    argv = [sys.executable, "-u", str(RUNNER)]

    if data_file is not None:
        argv += ["--file", str(data_file)]

    for flag in FLAGS:
        value = payload.get(flag.name)

        if flag.kind == "bool":
            if bool(value):
                argv.append(flag.cli)
            continue

        if flag.kind == "columns":
            joined = ",".join(_as_list(value))
            if joined:
                argv += [flag.cli, joined]
            continue

        if value in (None, "", []):
            continue

        if flag.kind == "int":
            argv += [flag.cli, str(int(value))]
        elif flag.kind == "float":
            argv += [flag.cli, str(float(value))]
        elif flag.kind == "choice":
            text = str(value)
            if text not in flag.choices:
                raise ValueError(f"{flag.label}: {text!r} is not one of {flag.choices}")
            if text != flag.default:  # the default needs no flag
                argv += [flag.cli, text]
        else:
            argv += [flag.cli, str(value)]

    return argv


# ---------------------------------------------------------------------------
# Artifacts
# ---------------------------------------------------------------------------


def _all_report_files() -> list[dict[str, Any]]:
    """Every file under ``reports/`` (excluding webui job logs), newest first."""
    if not REPORTS_DIR.is_dir():
        return []
    found = []
    for path in REPORTS_DIR.rglob("*"):
        if not path.is_file() or path.suffix.lower() == ".log":
            continue
        if LOG_DIR in path.parents:
            continue
        try:
            stat = path.stat()
        except OSError:
            continue
        found.append(
            {
                "path": path.relative_to(REPORTS_DIR).as_posix(),
                "name": path.name,
                "size": stat.st_size,
                "modified": stat.st_mtime,
                "kind": path.suffix.lstrip(".").lower(),
            }
        )
    found.sort(key=lambda a: a["modified"], reverse=True)
    return found


def artifacts_since(started_at: float) -> list[dict[str, Any]]:
    """Files under ``reports/`` touched by a run, newest first."""
    cutoff = started_at - 2  # tolerate filesystem timestamp granularity
    return [f for f in _all_report_files() if f["modified"] >= cutoff]


def _write_forecast_csv(result: dict[str, Any], upload_name: str) -> str:
    """Write history+forecast to a uniquely time-stamped CSV in ``reports/``.

    Time-stamped so re-running the forecast (different horizon, drop-edge
    toggles, etc.) never overwrites the previous download -- unlike the
    AutoML artifacts, which are named after the dataset and do overwrite.
    """
    base = Path(secure_filename(upload_name)).stem or "data"
    safe_col = re.sub(r"[^A-Za-z0-9]+", "_", result["value_col"]).strip("_") or "value"
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    name = f"forecast_{base}_{safe_col}_{stamp}_{uuid.uuid4().hex[:6]}.csv"
    path = REPORTS_DIR / name

    # Byte-valued columns (transfer volumes) are unreadable in raw units --
    # add a GB column alongside the exact byte figure, matching the on-screen table.
    is_bytes = bool(re.search(r"\(B\)|bytes?\b", result["value_col"], re.IGNORECASE))
    header = ["period", result["value_col"]]
    if is_bytes:
        header.append(re.sub(r"\(B\)", "(GB)", result["value_col"], flags=re.IGNORECASE))
    header.append("type")

    def row_values(row: dict[str, Any], kind: str) -> list[Any]:
        values = [row["period"], row["value"]]
        if is_bytes:
            values.append(round(row["value"] / 1e9, 3))
        values.append(kind)
        return values

    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.writer(fh)
        writer.writerow(header)
        for row in result["history"]:
            writer.writerow(row_values(row, "actual"))
        for row in result["forecast"]:
            writer.writerow(row_values(row, "forecast"))

    return name


def _safe_report_path(rel: str) -> Path:
    """Resolve ``rel`` inside ``reports/``, or abort -- no traversal out."""
    base = REPORTS_DIR.resolve()
    target = (base / rel).resolve()
    if target != base and base not in target.parents:
        abort(403, "Path outside the reports directory")
    if not target.is_file():
        abort(404, "No such artifact")
    return target


# ---------------------------------------------------------------------------
# App factory
# ---------------------------------------------------------------------------


def create_app() -> Flask:
    app = Flask(__name__)
    app.config["MAX_CONTENT_LENGTH"] = MAX_UPLOAD_BYTES
    app.json.sort_keys = False

    UPLOAD_DIR.mkdir(parents=True, exist_ok=True)
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    manager = JobManager()
    uploads: dict[str, Upload] = {}

    @app.get("/")
    def index():
        groups: dict[str, list[Flag]] = {}
        for flag in FLAGS:
            groups.setdefault(flag.group, []).append(flag)
        # The browser rebuilds the command preview client-side; give it the
        # same flag table build_argv() works from.
        flag_meta = [
            {"name": f.name, "cli": f.cli, "kind": f.kind, "default": f.default}
            for f in FLAGS
        ]
        return render_template(
            "index.html",
            groups=groups,
            allowed=sorted(ALLOWED_SUFFIXES),
            repo_root=str(REPO_ROOT),
            flag_meta=json.dumps(flag_meta),
        )

    @app.post("/api/upload")
    def upload():
        file = request.files.get("file")
        if file is None or not file.filename:
            return jsonify(error="No file part in the request"), 400

        name = secure_filename(file.filename)
        suffix = Path(name).suffix.lower()
        if suffix not in ALLOWED_SUFFIXES:
            return (
                jsonify(
                    error=f"Unsupported file type {suffix or '(none)'}. "
                    f"Allowed: {', '.join(sorted(ALLOWED_SUFFIXES))}"
                ),
                400,
            )

        upload_id = uuid.uuid4().hex[:12]
        dest = UPLOAD_DIR / f"{Path(name).stem}_{upload_id}{suffix}"
        file.save(dest)

        try:
            frame = _read_frame(dest)
        except Exception as exc:  # noqa: BLE001 - surface any parse failure verbatim
            dest.unlink(missing_ok=True)
            return jsonify(error=f"Could not read the file: {exc}"), 400

        record = Upload(
            id=upload_id,
            path=dest,
            original_name=file.filename,
            columns=[str(c) for c in frame.columns],
            rows=_row_count(dest),
            preview=_json_safe_preview(frame),
        )
        uploads[upload_id] = record

        return jsonify(
            upload_id=record.id,
            filename=record.original_name,
            stored_as=dest.name,
            columns=record.columns,
            rows=record.rows,
            preview=record.preview,
            dtypes={str(c): str(t) for c, t in frame.dtypes.items()},
        )

    @app.post("/api/forecast")
    def forecast():
        """Aggregate an upload to period totals and project it forward.

        Separate from /api/run on purpose: this answers "how much next
        month", which needs the rows collapsed to one value per period
        first. The AutoML path fits at row grain and cannot extrapolate.
        """
        payload = request.get_json(silent=True) or {}
        record = uploads.get(payload.get("upload_id") or "")
        if record is None:
            return jsonify(error="Upload a data file first"), 400

        date_col = payload.get("date_col")
        value_col = payload.get("value_col")
        if not date_col or not value_col:
            return jsonify(error="Pick both a period column and a value column"), 400

        spec = SeriesSpec(
            date_col=date_col,
            value_col=value_col,
            agg=payload.get("agg") or "sum",
            grain=payload.get("grain") or "auto",
            group_filters=payload.get("filters") or {},
            drop_first=bool(payload.get("drop_first")),
            drop_last=bool(payload.get("drop_last")),
        )
        if spec.agg not in {"sum", "mean", "max", "count"}:
            return jsonify(error=f"Unsupported aggregation: {spec.agg}"), 400

        horizon = payload.get("horizon") or 3
        try:
            horizon = max(1, min(int(horizon), 36))
        except (TypeError, ValueError):
            return jsonify(error="Horizon must be a whole number"), 400

        try:
            frame = _read_frame(record.path, nrows=None)
            result = run_forecast(frame, spec, horizon)
        except ValueError as exc:
            return jsonify(error=str(exc)), 400
        except Exception as exc:  # noqa: BLE001 - surface the real failure
            return jsonify(error=f"Forecast failed: {exc}"), 500

        result["csv_path"] = _write_forecast_csv(result, record.original_name)
        return jsonify(result)

    def _column_roles_from_payload(payload: dict[str, Any]) -> dict[str, list[str]]:
        return {
            "predict": _as_list(payload.get("predict")),
            "target": _as_list(payload.get("target")),
            "exclude": _as_list(payload.get("exclude")),
            "datetime_col": _as_list(payload.get("datetime_col")),
        }

    @app.post("/api/validate-columns")
    def validate_columns_route():
        """Advisory check: do the selected column roles make sense?

        Called live from the GUI as selections change, and again as a
        hard gate right before a run starts (see /api/run) -- a column
        used for two roles at once is rejected either way; softer LLM
        judgment calls are shown but don't block on their own here.
        """
        payload = request.get_json(silent=True) or {}
        record = uploads.get(payload.get("upload_id") or "")
        if record is None:
            return jsonify(error="Upload a data file first"), 400

        roles = _column_roles_from_payload(payload)
        if not any(roles.values()):
            return jsonify(ok=True, source="rules", issues=[])

        try:
            frame = _read_frame(record.path, nrows=2000)
        except Exception as exc:  # noqa: BLE001
            return jsonify(error=f"Could not read the file: {exc}"), 400

        result = validate_columns(frame, roles)
        return jsonify(result.to_dict())

    @app.post("/api/run")
    def run():
        payload = request.get_json(silent=True) or {}
        upload_id = payload.get("upload_id")
        run_all_demos = bool(payload.get("all"))

        data_file: Path | None = None
        record = None
        if upload_id:
            record = uploads.get(upload_id)
            if record is None:
                return jsonify(error="Unknown upload -- re-upload the file"), 400
            data_file = record.path
        elif not run_all_demos:
            return (
                jsonify(
                    error="Upload a data file, or tick 'Run all built-in demos' "
                    "to run without one."
                ),
                400,
            )

        has_target = bool(_as_list(payload.get("predict")) or _as_list(payload.get("target")))
        if data_file is not None and not run_all_demos and not has_target:
            return jsonify(error="Pick at least one target column to predict."), 400

        if record is not None:
            roles = _column_roles_from_payload(payload)
            if any(roles.values()):
                try:
                    frame = _read_frame(record.path, nrows=2000)
                    check = validate_columns(frame, roles)
                except Exception:
                    check = None  # don't block a run over a validator-side failure
                if check is not None and not check.ok:
                    blocking = [i for i in check.issues if i.severity == "block"]
                    return (
                        jsonify(
                            error="Column selection conflict: " + " ".join(i.message for i in blocking),
                            column_issues=check.to_dict(),
                        ),
                        400,
                    )

        try:
            argv = build_argv(payload, data_file)
        except (ValueError, TypeError) as exc:
            return jsonify(error=f"Invalid option: {exc}"), 400

        try:
            job = manager.start(argv)
        except RuntimeError as exc:
            return jsonify(error=str(exc)), 409

        return jsonify(job_id=job.id, command=job.command)

    @app.get("/api/job/<job_id>")
    def job_status(job_id: str):
        job = manager.get(job_id)
        if job is None:
            return jsonify(error="Unknown job"), 404
        cursor = request.args.get("cursor", type=int, default=0)
        lines, next_cursor = job.slice_from(cursor)
        return jsonify(
            job_id=job.id,
            status=job.status,
            returncode=job.returncode,
            command=job.command,
            elapsed=round(time.time() - job.started_at, 1),
            lines=lines,
            cursor=next_cursor,
            log_file=str(job.log_path.relative_to(REPO_ROOT)),
            artifacts=artifacts_since(job.started_at),
        )

    @app.post("/api/job/<job_id>/stop")
    def job_stop(job_id: str):
        job = manager.get(job_id)
        if job is None:
            return jsonify(error="Unknown job"), 404
        stopped = manager.stop(job)
        return jsonify(stopped=stopped, status=job.status)

    @app.get("/api/jobs")
    def job_list():
        return jsonify(
            jobs=[
                {
                    "job_id": j.id,
                    "status": j.status,
                    "started_at": j.started_at,
                    "command": j.command,
                }
                for j in manager.recent()
            ]
        )

    @app.get("/api/reports")
    def reports_list():
        """All files ever written to reports/ -- past runs, not just the live job."""
        limit = request.args.get("limit", type=int, default=300)
        limit = max(1, min(limit, 2000))
        all_reports = _all_report_files()
        return jsonify(
            reports=all_reports[:limit],
            total=len(all_reports),
            shown=min(limit, len(all_reports)),
        )

    @app.get("/api/artifact")
    def artifact():
        rel = request.args.get("path", "")
        if not rel:
            abort(400, "Missing path")
        target = _safe_report_path(rel)
        inline = request.args.get("inline") == "1"
        return send_file(target, as_attachment=not inline)

    @app.get("/api/health")
    def health():
        active = manager.active_job
        return jsonify(
            ok=True,
            repo_root=str(REPO_ROOT),
            runner=str(RUNNER),
            runner_present=RUNNER.is_file(),
            active_job=active.id if active and active.status == "running" else None,
        )

    return app


# Module-level instance for `waitress-serve acdso.webui.app:app` / any other
# WSGI runner that expects to import an app object rather than call a
# factory. `start.py` uses create_app() directly and does not touch this.
app = create_app()
