# JupyterHub Docker Image for UJ AI Club
# Includes DockerSpawner, nbgrader, and custom authenticator

FROM jupyterhub/jupyterhub:4.0

USER root

# Install Python dependencies
RUN pip install --no-cache-dir \
    dockerspawner==12.* \
    jupyterhub-idle-culler \
    nbgrader==0.9.* \
    PyJWT>=2.0.0 \
    requests

# Create directories
RUN mkdir -p /srv/jupyterhub /data /srv/nbgrader/exchange \
    && chmod 777 /srv/nbgrader/exchange

# Copy configuration files
COPY jupyterhub_config.py /srv/jupyterhub/jupyterhub_config.py
COPY custom_authenticator.py /srv/jupyterhub/custom_authenticator.py

# nbgrader configuration for the hub
COPY nbgrader_config_hub.py /etc/jupyter/nbgrader_config.py

WORKDIR /srv/jupyterhub

# Expose ports
EXPOSE 8000

# Volume for persistent data
VOLUME ["/data", "/srv/nbgrader/exchange"]

# Start JupyterHub
CMD ["jupyterhub", "-f", "/srv/jupyterhub/jupyterhub_config.py"]
