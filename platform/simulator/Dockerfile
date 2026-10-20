# Debian 13 userspace, ARM64. This is a test harness, not a dArkOS system image.
FROM debian:trixie-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 libfontconfig1 libfreetype6 fonts-dejavu-core \
    libinput10 libxkbcommon0 libudev1 libdrm2 libgbm1 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --uid 10001 --create-home tester
USER 10001:10001
WORKDIR /workspace
ENV SLINT_SCALE_FACTOR=1 SLINT_RENDERER=software
ENTRYPOINT ["python3", "/workspace/platform/simulator/suite.py"]
