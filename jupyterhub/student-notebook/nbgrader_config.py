"""
nbgrader Configuration for Student Notebooks
"""

from nbgrader.apps import NbGraderAPI

c = get_config()

# Course configuration
c.CourseDirectory.course_id = 'ujaiclub'

# Exchange configuration
c.Exchange.root = '/srv/nbgrader/exchange'

# Student settings
c.CourseDirectory.student_id = '*'

# Submission settings
c.SubmitApp.strict = True

# Disable direct database access for students
c.CourseDirectory.db_url = ''

# Feedback settings
c.GenerateFeedbackApp.make_hierarchical_pdf = False
