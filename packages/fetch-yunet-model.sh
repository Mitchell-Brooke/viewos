#!/bin/bash
# Fetch the YuNet face detection model into the Debian package tree.
#
# The model is not committed to the repository: it is a 227 KiB binary blob
# that upstream does not version usefully, and the licence is best carried
# alongside it rather than duplicated in git. This script downloads it from a
# pinned OpenCV Zoo release tag and verifies the hash, so the result is the
# same bytes on every machine and a compromised or moved URL fails loudly
# instead of quietly installing something else.
#
# Usage:
#   packages/fetch-yunet-model.sh [destination-directory]
#
# Default destination is packages/viewos-face-daemon/usr/share/viewos/models,
# which is where the daemon expects it at runtime.

set -euo pipefail

# Pinned upstream. Do not change these without re-verifying the hash.
readonly ZOO_REPO="opencv/opencv_zoo"
readonly ZOO_TAG="4.10.0"
readonly MODEL_PATH_IN_REPO="models/face_detection_yunet/face_detection_yunet_2023mar.onnx"
readonly MODEL_FILENAME="face_detection_yunet_2023mar.onnx"

# sha256 and size of the file as published at ZOO_TAG above. Verified against
# the Git LFS pointer in that tag, which carries the same digest.
readonly MODEL_SHA256="8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4"
readonly MODEL_SIZE="232589"

# opencv_zoo stores its models in Git LFS, so the raw.githubusercontent.com URL
# returns a ~130 byte pointer file rather than the model. media.githubusercontent
# is the endpoint that resolves LFS content.
readonly MODEL_URL="https://media.githubusercontent.com/media/${ZOO_REPO}/${ZOO_TAG}/${MODEL_PATH_IN_REPO}"

REPO_ROOT="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
DEST_DIR="${1:-${REPO_ROOT}/packages/viewos-face-daemon/usr/share/viewos/models}"

die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required"

TARGET="${DEST_DIR}/${MODEL_FILENAME}"

# Skip the download if the file is already correct. This makes the script
# cheap to run repeatedly from CI.
if [ -f "$TARGET" ]; then
    existing="$(sha256sum "$TARGET" | cut -d' ' -f1)"
    if [ "$existing" = "$MODEL_SHA256" ]; then
        info "model already present and correct: ${TARGET}"
        exit 0
    fi
    info "existing model has the wrong hash, refetching"
    rm -f "$TARGET"
fi

mkdir -p "$DEST_DIR"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

info "downloading YuNet from ${ZOO_REPO}@${ZOO_TAG}"
curl --fail --location --silent --show-error \
     --retry 5 --retry-all-errors --retry-delay 2 \
     --output "${TMP}/${MODEL_FILENAME}" \
     "$MODEL_URL" \
    || die "download failed: ${MODEL_URL}"

actual_size="$(stat -c%s "${TMP}/${MODEL_FILENAME}")"
[ "$actual_size" = "$MODEL_SIZE" ] \
    || die "expected ${MODEL_SIZE} bytes, got ${actual_size}. This usually means a Git LFS \
pointer was downloaded instead of the model."

actual_sha256="$(sha256sum "${TMP}/${MODEL_FILENAME}" | cut -d' ' -f1)"
[ "$actual_sha256" = "$MODEL_SHA256" ] \
    || die "hash mismatch.
  expected ${MODEL_SHA256}
  got      ${actual_sha256}
The upstream tag may have been repointed. Do not bypass this check: an
unverified model file is a remote code execution path, since it is parsed by
OpenCV's DNN loader."

install -m 0644 "${TMP}/${MODEL_FILENAME}" "$TARGET"

# The MIT licence must travel with the model: the daemon is GPL, but the
# weights are separately licensed, and the attribution is a licence condition.
cat > "${DEST_DIR}/LICENSE.YuNet" <<'EOF'
YuNet face detection model
==========================

Source:   https://github.com/opencv/opencv_zoo
Path:     models/face_detection_yunet/face_detection_yunet_2023mar.onnx
Tag:      4.10.0
Author:   Shiqi Yu
Paper:    "YuNet: A Lightweight CNN for Real-time Face Detection",
          arXiv:2105.15141

MIT License

Copyright (c) 2020 Shiqi Yu <shiqi.yu@gmail.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

info "installed ${TARGET}"
info "  sha256 ${MODEL_SHA256}"
info "  ${MODEL_SIZE} bytes"
