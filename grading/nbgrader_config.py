c = get_config()

c.CourseDirectory.course_id = 'ujaiclub'
c.CourseDirectory.root = '/srv/nbgrader/course'

c.Exchange.root = '/srv/nbgrader/exchange'

c.CourseDirectory.db_url = 'sqlite:////data/nbgrader.db'

c.ClearSolutions.code_stub = {
    'python': '# YOUR CODE HERE\nraise NotImplementedError()',
}

c.ExecutePreprocessor.timeout = 120
c.ExecutePreprocessor.interrupt_on_timeout = True
c.Execute.allow_errors = False
