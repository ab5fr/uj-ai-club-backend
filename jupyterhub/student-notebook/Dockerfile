
FROM quay.io/jupyter/scipy-notebook:2025-06-23

USER root

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir \
    nbgrader==0.9.* \
    jupyterlab-git

RUN mkdir -p /srv/nbgrader/exchange \
    && chmod 777 /srv/nbgrader/exchange

RUN mkdir -p /etc/jupyter
COPY nbgrader_config.py /etc/jupyter/nbgrader_config.py
COPY jupyter_server_config.py /etc/jupyter/jupyter_server_config.py

RUN mkdir -p /opt/conda/share/jupyter/lab/settings
COPY overrides.json /opt/conda/share/jupyter/lab/settings/overrides.json

RUN mkdir -p /home/jovyan/.jupyter \
    && echo "c.ServerApp.token = ''" >> /home/jovyan/.jupyter/jupyter_server_config.py \
    && echo "c.ServerApp.password = ''" >> /home/jovyan/.jupyter/jupyter_server_config.py \
    && chown -R jovyan:users /home/jovyan/.jupyter

RUN mkdir -p /home/jovyan/work && chown jovyan:users /home/jovyan/work

USER jovyan

WORKDIR /home/jovyan/work

CMD ["start-notebook.sh", "--ServerApp.token=''"]
