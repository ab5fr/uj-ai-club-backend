#!/usr/bin/env python3
"""CLI wrapper for setting up nbgrader assignments from source notebooks."""

import re
import shutil
import sys
from pathlib import Path

from notebook_processor import process_notebook_file


def setup_nbgrader_assignment(source_notebook, assignment_name, course_dir="/srv/nbgrader/course"):
    source_dir = Path(course_dir) / "source" / assignment_name
    release_dir = Path(course_dir) / "release" / assignment_name

    source_dir.mkdir(parents=True, exist_ok=True)
    release_dir.mkdir(parents=True, exist_ok=True)

    original_filename = Path(source_notebook).name
    notebook_name = re.sub(r"^[a-f0-9-]{36}_", "", original_filename)

    source_dest = source_dir / notebook_name
    shutil.copy2(source_notebook, source_dest)
    print(f"Copied source to: {source_dest}")

    release_dest = release_dir / notebook_name
    process_notebook_file(str(source_dest), str(release_dest))

    return str(release_dest)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python process_notebook.py <source_notebook> <assignment_name>")
        sys.exit(1)

    result = setup_nbgrader_assignment(sys.argv[1], sys.argv[2])
    print(f"Assignment ready: {result}")
