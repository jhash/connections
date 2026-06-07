#!/bin/sh
set -eu

git config --global --add safe.directory /repo
git -C /repo remote set-url origin git@github.com:jhash/connections.git 2>/dev/null || true

if [ -f /run/secrets/connections_GITHUB_DEPLOY_KEY ]; then
    mkdir -p /root/.ssh
    cp /run/secrets/connections_GITHUB_DEPLOY_KEY /root/.ssh/id_ed25519
    chmod 600 /root/.ssh/id_ed25519
    ssh-keyscan -H github.com >> /root/.ssh/known_hosts 2>/dev/null
fi

exec "$@"
