
FROM python:3.13-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir \
    nbgrader==0.9.* \
    watchdog \
    requests \
    flask \
    docker

COPY grading_service.py /app/grading_service.py
COPY notebook_processor.py /app/notebook_processor.py
COPY nbgrader_config.py /etc/jupyter/nbgrader_config.py

RUN mkdir -p /srv/nbgrader/exchange /srv/nbgrader/course /data

EXPOSE 9100

CMD ["python", "/app/grading_service.py"]
