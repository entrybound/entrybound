#!/usr/bin/env bash
# Authoring helper: (re)generate the hash-locked PyPI requirement sets used by the F03 pip items.
set -euo pipefail
cd "$(dirname "$0")"
PY=/root/eb-research/venv/bin/python
[[ -f ../locks/pypi-scientific-stack.txt ]] || $PY -B make_pip_lock.py pypi-scientific-stack \
  numpy scipy pandas matplotlib scikit-learn sympy pillow networkx seaborn
[[ -f ../locks/pypi-web-service-stack.txt ]] || $PY -B make_pip_lock.py pypi-web-service-stack \
  django djangorestframework celery boto3 sqlalchemy alembic "psycopg[binary]" pydantic fastapi \
  "uvicorn[standard]" httpx requests jinja2 gunicorn sentry-sdk cryptography
grep -c '^[a-z0-9]' ../locks/*.txt
sha256sum ../locks/*.txt
