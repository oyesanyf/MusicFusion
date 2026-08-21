#!/usr/bin/env python
"""Launch the ACDSO web GUI.

    python start.py                     # http://127.0.0.1:5000, opens a browser
    python start.py --port 8000         # different port
    python start.py --no-browser        # don't open a browser
    python start.py --host 0.0.0.0      # listen on all interfaces (see warning)

The GUI is a front-end over ``run_acdso.py``: upload a data file, choose a
target column, toggle any CLI flag, run, and collect the reports and
predictions the pipeline writes to ``reports/``.
"""

from __future__ import annotations

import argparse
import sys
import threading
import webbrowser
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent
SRC = REPO_ROOT / "src"

# Run straight from a checkout without needing `pip install -e .` first.
if SRC.is_dir() and str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))


def _check_dependencies() -> None:
    missing = []
    for module, pip_name in (("flask", "flask"), ("pandas", "pandas")):
        try:
            __import__(module)
        except ImportError:
            missing.append(pip_name)
    if missing:
        print("Missing dependencies for the web GUI: " + ", ".join(missing))
        print("Install them with:\n")
        print(f"    {Path(sys.executable).name} -m pip install " + " ".join(missing))
        sys.exit(1)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Start the ACDSO web GUI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--host", default="127.0.0.1",
                        help="Interface to bind (default: 127.0.0.1, localhost only)")
    parser.add_argument("--port", type=int, default=5000, help="Port (default: 5000)")
    parser.add_argument("--no-browser", action="store_true",
                        help="Do not open a browser window on startup")
    parser.add_argument("--debug", action="store_true",
                        help="Flask debug mode with auto-reload")
    args = parser.parse_args(argv)

    _check_dependencies()

    runner = REPO_ROOT / "run_acdso.py"
    if not runner.is_file():
        print(f"Cannot find {runner}. Run start.py from the ACDSO repository root.")
        return 1

    from acdso.webui import create_app

    app = create_app()
    shown_host = "127.0.0.1" if args.host in ("0.0.0.0", "::") else args.host
    url = f"http://{shown_host}:{args.port}/"

    print("=" * 72)
    print("  ACDSO web GUI")
    print("=" * 72)
    print(f"  repo      : {REPO_ROOT}")
    print(f"  runner    : {runner.name}")
    print(f"  uploads   : {REPO_ROOT / 'uploads'}")
    print(f"  reports   : {REPO_ROOT / 'reports'}")
    print(f"  serving   : {url}")
    if args.host not in ("127.0.0.1", "localhost"):
        print("  WARNING   : bound beyond localhost. Anyone who can reach this port")
        print("              can upload data and start training runs on this machine.")
    print("=" * 72)
    print("  Ctrl+C to stop.\n")

    if not args.no_browser:
        threading.Timer(1.0, lambda: webbrowser.open(url)).start()

    # threaded=True so log polling stays responsive while a run is streaming.
    app.run(host=args.host, port=args.port, debug=args.debug,
            threaded=True, use_reloader=args.debug)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
