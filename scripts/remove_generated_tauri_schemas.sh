#!/usr/bin/env bash
set -euo pipefail
rm -f \
  apps/desktop/src-tauri/gen/schemas/acl-manifests.json \
  apps/desktop/src-tauri/gen/schemas/capabilities.json \
  apps/desktop/src-tauri/gen/schemas/desktop-schema.json \
  apps/desktop/src-tauri/gen/schemas/linux-schema.json
