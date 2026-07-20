# Jupyter server settings for student notebooks
c = get_config()

c.ServerApp.token = ""
c.ServerApp.password = ""

# Note: JupyterLab autosave is controlled by lab/settings/overrides.json
# (Document Manager), not FileContentsManager — that trait no longer exists.
