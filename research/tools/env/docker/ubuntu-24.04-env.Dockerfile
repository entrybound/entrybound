# Entrybound research: container used to fingerprint the pinned ubuntu:24.04 environment.
#
# Built by research/tools/env/capture_docker.py (never by hand). The base image is
# pinned by OCI image-index digest, and every package comes from a fixed
# snapshot.ubuntu.com timestamp so rebuilds install byte-identical .debs.
#
# Only python3 is added (the fingerprinting script needs an interpreter). No
# archivers beyond what the base image ships are installed, so the tools section
# of the fingerprint describes the stock image.
#
# ca-certificates.crt is bind-mounted for the build step only, to let apt reach
# the HTTPS-only snapshot service. Package integrity is still enforced by apt's
# InRelease signature check against the keyring shipped in the base image; the
# CA bundle never lands in an image layer.
# Defaults must equal the constants in capture_docker.py (the driver checks this).
ARG BASE_IMAGE=docker.io/library/ubuntu:24.04@sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254
FROM ${BASE_IMAGE}

ARG APT_SNAPSHOT=20260912T000000Z
ARG PACKAGES=python3

RUN --mount=type=bind,source=ca-certificates.crt,target=/run/eb-transport-ca.crt \
    set -eu; \
    test -n "${APT_SNAPSHOT}"; test -n "${PACKAGES}"; \
    sed -i \
      -e "s#http://archive.ubuntu.com/ubuntu/#https://snapshot.ubuntu.com/ubuntu/${APT_SNAPSHOT}/#" \
      -e "s#http://security.ubuntu.com/ubuntu/#https://snapshot.ubuntu.com/ubuntu/${APT_SNAPSHOT}/#" \
      /etc/apt/sources.list.d/ubuntu.sources; \
    APT="apt-get -o Acquire::https::CAInfo=/run/eb-transport-ca.crt -o Acquire::Retries=3"; \
    $APT update; \
    mkdir -p /eb-provenance; \
    $APT install --print-uris -qq --no-install-recommends $PACKAGES > /eb-provenance/apt-print-uris.txt; \
    $APT install -y --download-only --no-install-recommends $PACKAGES; \
    ( cd /var/cache/apt/archives && for f in *.deb; do \
        printf '%s\t%s\t%s\n' "$f" "$(stat -c %s "$f")" "$(sha256sum "$f" | cut -d' ' -f1)"; \
      done ) > /eb-provenance/apt-debs.tsv; \
    DEBIAN_FRONTEND=noninteractive $APT install -y --no-install-recommends $PACKAGES; \
    dpkg-query -W -f='${Package}\t${Version}\t${Architecture}\n' | LC_ALL=C sort > /eb-provenance/dpkg-installed.tsv; \
    printf '%s\n' "${APT_SNAPSHOT}" > /eb-provenance/apt-snapshot; \
    rm -rf /var/lib/apt/lists/*
