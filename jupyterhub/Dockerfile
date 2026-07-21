
FROM jupyterhub/jupyterhub:5.5.0

USER root

RUN pip install --no-cache-dir \
    dockerspawner==12.* \
    jupyterhub-idle-culler \
    nbgrader==0.9.* \
    PyJWT>=2.0.0 \
    requests

RUN mkdir -p /srv/jupyterhub /data /srv/nbgrader/exchange \
    && chmod 777 /srv/nbgrader/exchange

COPY jupyterhub_config.py /srv/jupyterhub/jupyterhub_config.py
COPY custom_authenticator.py /srv/jupyterhub/custom_authenticator.py

COPY nbgrader_config_hub.py /etc/jupyter/nbgrader_config.py

WORKDIR /srv/jupyterhub

EXPOSE 8000

VOLUME ["/data", "/srv/nbgrader/exchange"]

CMD ["jupyterhub", "-f", "/srv/jupyterhub/jupyterhub_config.py"]
