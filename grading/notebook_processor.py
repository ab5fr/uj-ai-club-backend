from __future__ import annotations

import json
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

SOLUTION_BEGIN_MARKERS = ("### BEGIN SOLUTION", "# BEGIN SOLUTION")
SOLUTION_END_MARKERS = ("### END SOLUTION", "# END SOLUTION")
HIDDEN_BEGIN_MARKERS = ("### BEGIN HIDDEN TESTS", "# BEGIN HIDDEN TESTS")
HIDDEN_END_MARKERS = ("### END HIDDEN TESTS", "# END HIDDEN TESTS")

def _contains_marker(source: str, markers: tuple[str, ...]) -> bool:
    return any(marker in source for marker in markers)

def remove_solution_code(source: str) -> str:
    lines = source.split("\n")
    result: list[str] = []
    in_solution = False
    indent = ""

    for line in lines:
        if _contains_marker(line, SOLUTION_BEGIN_MARKERS):
            in_solution = True
            indent = line[: len(line) - len(line.lstrip())]
            result.append(f"{indent}# YOUR CODE HERE")
            result.append(f"{indent}raise NotImplementedError()")
            continue
        if _contains_marker(line, SOLUTION_END_MARKERS):
            in_solution = False
            continue
        if not in_solution:
            result.append(line)

    return "\n".join(result)

def remove_hidden_tests(source: str) -> str:
    lines = source.split("\n")
    result: list[str] = []
    in_hidden = False

    for line in lines:
        if _contains_marker(line, HIDDEN_BEGIN_MARKERS):
            in_hidden = True
            continue
        if _contains_marker(line, HIDDEN_END_MARKERS):
            in_hidden = False
            continue
        if not in_hidden:
            result.append(line)

    return "\n".join(result)

def _cell_source_to_str(source) -> str:
    if isinstance(source, list):
        return "".join(source)
    return source or ""

def _set_cell_source(cell: dict, source: str) -> None:
    cell["source"] = source

def process_notebook_cells(notebook: dict) -> dict:
    for cell in notebook.get("cells", []):
        if cell.get("cell_type") != "code":
            if "outputs" in cell:
                cell["outputs"] = []
            if "execution_count" in cell:
                cell["execution_count"] = None
            continue

        source = _cell_source_to_str(cell.get("source", ""))

        if _contains_marker(source, SOLUTION_BEGIN_MARKERS):
            source = remove_solution_code(source)
            _set_cell_source(cell, source)

        if _contains_marker(source, HIDDEN_BEGIN_MARKERS):
            source = _cell_source_to_str(cell.get("source", ""))
            source = remove_hidden_tests(source)
            _set_cell_source(cell, source)

        if "outputs" in cell:
            cell["outputs"] = []
        if "execution_count" in cell:
            cell["execution_count"] = None

    return notebook

def process_notebook_file(source_path: str | Path, output_path: str | Path) -> None:
    source_path = Path(source_path)
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(source_path, encoding="utf-8") as handle:
        notebook = json.load(handle)

    process_notebook_cells(notebook)

    with open(output_path, "w", encoding="utf-8") as handle:
        json.dump(notebook, handle, indent=1)

    logger.info("Processed notebook saved to: %s", output_path)
