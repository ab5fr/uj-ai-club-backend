"""
nbgrader Configuration for JupyterHub (Admin/Grading)
"""

c = get_config()

# Course configuration
c.CourseDirectory.course_id = 'ujaiclub'
c.CourseDirectory.root = '/srv/nbgrader/course'

# Exchange configuration
c.Exchange.root = '/srv/nbgrader/exchange'

# Database for grades
c.CourseDirectory.db_url = 'sqlite:////data/nbgrader.db'

# Assignment settings
c.ClearSolutions.code_stub = {
    'python': '# YOUR CODE HERE\nraise NotImplementedError()',
}

# Autograder settings
c.ExecutePreprocessor.timeout = 120  # 2 minutes per cell
c.ExecutePreprocessor.interrupt_on_timeout = True
c.Execute.allow_errors = False
