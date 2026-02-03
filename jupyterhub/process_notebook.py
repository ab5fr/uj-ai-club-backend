#!/usr/bin/env python3
"""
Process uploaded notebooks for nbgrader.
This script:
1. Takes a source notebook with solutions and hidden tests
2. Generates a student version (removes solutions, keeps only visible tests)
3. Sets up the nbgrader course structure
"""

import json
import os
import re
import shutil
import sys
from pathlib import Path

def remove_solution_code(source):
    """Remove code between ### BEGIN SOLUTION and ### END SOLUTION markers."""
    lines = source.split('\n')
    result = []
    in_solution = False
    
    for line in lines:
        if '### BEGIN SOLUTION' in line or '# BEGIN SOLUTION' in line:
            in_solution = True
            # Add a placeholder
            result.append('    # YOUR CODE HERE')
            result.append('    raise NotImplementedError()')
            continue
        elif '### END SOLUTION' in line or '# END SOLUTION' in line:
            in_solution = False
            continue
        
        if not in_solution:
            result.append(line)
    
    return '\n'.join(result)

def remove_hidden_tests(source):
    """Remove code between ### BEGIN HIDDEN TESTS and ### END HIDDEN TESTS markers."""
    lines = source.split('\n')
    result = []
    in_hidden = False
    
    for line in lines:
        if '### BEGIN HIDDEN TESTS' in line or '# BEGIN HIDDEN TESTS' in line:
            in_hidden = True
            continue
        elif '### END HIDDEN TESTS' in line or '# END HIDDEN TESTS' in line:
            in_hidden = False
            continue
        
        if not in_hidden:
            result.append(line)
    
    return '\n'.join(result)

def process_notebook(source_path, output_path):
    """
    Process a source notebook to create a student version.
    
    Args:
        source_path: Path to the source notebook with solutions
        output_path: Path where the student notebook will be saved
    """
    with open(source_path, 'r', encoding='utf-8') as f:
        notebook = json.load(f)
    
    # Process each cell
    for cell in notebook.get('cells', []):
        if cell.get('cell_type') == 'code':
            source = ''.join(cell.get('source', []))
            
            # Check if this is a solution cell
            if '### BEGIN SOLUTION' in source or '# BEGIN SOLUTION' in source:
                processed = remove_solution_code(source)
                cell['source'] = processed.split('\n')
                # Add newlines back except for last line
                cell['source'] = [line + '\n' for line in cell['source'][:-1]] + [cell['source'][-1]]
            
            # Check if this has hidden tests
            if '### BEGIN HIDDEN TESTS' in source or '# BEGIN HIDDEN TESTS' in source:
                processed = remove_hidden_tests(source)
                cell['source'] = processed.split('\n')
                cell['source'] = [line + '\n' for line in cell['source'][:-1]] + [cell['source'][-1]]
        
        # Clear outputs
        if 'outputs' in cell:
            cell['outputs'] = []
        if 'execution_count' in cell:
            cell['execution_count'] = None
    
    # Ensure output directory exists
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    
    # Write the processed notebook
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(notebook, f, indent=1)
    
    print(f"Created student notebook: {output_path}")

def setup_nbgrader_assignment(source_notebook, assignment_name, course_dir='/srv/nbgrader/course'):
    """
    Set up an nbgrader assignment from a source notebook.
    
    Args:
        source_notebook: Path to the source notebook
        assignment_name: Name of the assignment
        course_dir: Base directory for nbgrader course
    """
    # Create directory structure
    source_dir = Path(course_dir) / 'source' / assignment_name
    release_dir = Path(course_dir) / 'release' / assignment_name
    
    source_dir.mkdir(parents=True, exist_ok=True)
    release_dir.mkdir(parents=True, exist_ok=True)
    
    # Copy source notebook
    notebook_name = Path(source_notebook).name
    # Remove UUID prefix if present
    if '_' in notebook_name:
        parts = notebook_name.split('_', 1)
        if len(parts[0]) == 36:  # UUID length
            notebook_name = parts[1]
    
    source_dest = source_dir / notebook_name
    shutil.copy2(source_notebook, source_dest)
    print(f"Copied source to: {source_dest}")
    
    # Generate student version
    release_dest = release_dir / notebook_name
    process_notebook(str(source_dest), str(release_dest))
    
    return str(release_dest)

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python process_notebook.py <source_notebook> <assignment_name>")
        sys.exit(1)
    
    source_notebook = sys.argv[1]
    assignment_name = sys.argv[2]
    
    result = setup_nbgrader_assignment(source_notebook, assignment_name)
    print(f"Assignment ready: {result}")
