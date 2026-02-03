# Student Notebook Docker Image for UJ AI Club
# Based on Jupyter Docker Stacks with nbgrader

FROM jupyter/scipy-notebook:latest

USER root

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install nbgrader and other Python packages
# nbgrader 0.9+ uses JupyterLab extensions, not classic nbextensions
RUN pip install --no-cache-dir \
    nbgrader==0.9.* \
    jupyterlab-git

# Create nbgrader exchange directory
RUN mkdir -p /srv/nbgrader/exchange \
    && chmod 777 /srv/nbgrader/exchange

# Create nbgrader config
RUN mkdir -p /etc/jupyter
COPY nbgrader_config.py /etc/jupyter/nbgrader_config.py

# Configure Jupyter settings
RUN mkdir -p /home/jovyan/.jupyter
RUN echo "c.ServerApp.token = ''" >> /home/jovyan/.jupyter/jupyter_server_config.py \
    && echo "c.ServerApp.password = ''" >> /home/jovyan/.jupyter/jupyter_server_config.py \
    && echo "c.ServerApp.allow_origin = '*'" >> /home/jovyan/.jupyter/jupyter_server_config.py \
    && chown -R jovyan:users /home/jovyan/.jupyter

# Create work directory
RUN mkdir -p /home/jovyan/work && chown jovyan:users /home/jovyan/work

# Switch back to notebook user
USER jovyan

# Set working directory
WORKDIR /home/jovyan/work

# Default command
CMD ["start-notebook.sh", "--ServerApp.token=''"]
