from nbgrader.apps import NbGraderAPI

c = get_config()

c.CourseDirectory.course_id = 'ujaiclub'

c.Exchange.root = '/srv/nbgrader/exchange'

c.CourseDirectory.student_id = '*'

c.SubmitApp.strict = True

c.CourseDirectory.db_url = ''

c.GenerateFeedbackApp.make_hierarchical_pdf = False
