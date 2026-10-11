#!/usr/bin/env bash
set -euo pipefail
# Resolve a tag once before any release/publish job can run. Do not treat an
# operator-supplied tag label as evidence that the event checkout is that tag.
[[ "$RELEASE_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]
source_sha=$(git rev-parse --verify "refs/tags/${RELEASE_TAG}^{commit}")
[[ "$source_sha" =~ ^[0-9a-f]{40}$|^[0-9a-f]{64}$ ]]
if [[ "$EVENT_NAME" == push || "$NPM_ONLY" != true ]]; then
  event_sha=$(git rev-parse --verify "${EVENT_SHA}^{commit}")
  if [[ "$event_sha" != "$source_sha" ]]; then
    echo 'Release tag and event commit differ; refusing the full release.' >&2
    exit 1
  fi
fi
printf 'commit=%s\n' "$source_sha" >> "$GITHUB_OUTPUT"
