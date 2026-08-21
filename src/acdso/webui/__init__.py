"""Web GUI for ACDSO.

A thin Flask front-end over the ``run_acdso.py`` CLI: upload a data file,
pick a target, toggle any of the CLI flags, and watch the run stream back
live.  Nothing here reimplements the pipeline -- every run is the exact
same command line you would have typed.

Start it with ``python start.py`` from the repository root.
"""

from .app import create_app

__all__ = ["create_app"]
