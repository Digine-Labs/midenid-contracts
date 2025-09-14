#!/usr/bin/env bash

set -euo pipefail

# Starts the node via scripts/start_node.sh, waits for it to be ready,
# then runs tests and cleans up the node process on exit.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LOG_DIR="${REPO_ROOT}/.logs"
NODE_LOG="${LOG_DIR}/node.out"

RPC_HOST="${RPC_HOST:-127.0.0.1}"
RPC_PORT="${RPC_PORT:-57291}"
READY_TIMEOUT_SEC="${READY_TIMEOUT_SEC:-120}"

mkdir -p "${LOG_DIR}"

cleanup() {
  local status=$?
  if [[ -n "${NODE_PID:-}" ]] && kill -0 "${NODE_PID}" >/dev/null 2>&1; then
    # Kill the entire process group for robustness
    local pgid
    pgid=$(ps -o pgid= "${NODE_PID}" 2>/dev/null | tr -d ' ' || true)
    if [[ -n "${pgid}" ]]; then
      kill -TERM -"${pgid}" 2>/dev/null || true
      # Give it a moment to exit gracefully
      sleep 2
      kill -KILL -"${pgid}" 2>/dev/null || true
    else
      kill -TERM "${NODE_PID}" 2>/dev/null || true
      sleep 2
      kill -KILL "${NODE_PID}" 2>/dev/null || true
    fi
  fi
  exit ${status}
}
trap cleanup EXIT INT TERM

echo "[info] Starting node..." | tee "${LOG_DIR}/run.log"

cd "${REPO_ROOT}"

# Start node in its own process group and capture PID
setsid bash "${SCRIPT_DIR}/start_node.sh" >"${NODE_LOG}" 2>&1 &
NODE_PID=$!

echo "[info] Node PID: ${NODE_PID}" | tee -a "${LOG_DIR}/run.log"
echo "[info] Node logs: ${NODE_LOG}" | tee -a "${LOG_DIR}/run.log"

# Wait for RPC port to accept connections, or fail if the process exits
echo -n "[info] Waiting for RPC at ${RPC_HOST}:${RPC_PORT} " | tee -a "${LOG_DIR}/run.log"
for ((i=1; i<=READY_TIMEOUT_SEC; i++)); do
  if ! kill -0 "${NODE_PID}" >/dev/null 2>&1; then
    echo -e "\n[error] Node process exited early. Tail of logs:" | tee -a "${LOG_DIR}/run.log"
    tail -n 100 "${NODE_LOG}" || true
    exit 1
  fi
  if bash -c "/bin/echo >/dev/tcp/${RPC_HOST}/${RPC_PORT}" >/dev/null 2>&1; then
    echo "\n[info] Node is ready." | tee -a "${LOG_DIR}/run.log"
    break
  fi
  echo -n "."
  sleep 1
  if (( i == READY_TIMEOUT_SEC )); then
    echo -e "\n[error] Timeout waiting for node readiness after ${READY_TIMEOUT_SEC}s" | tee -a "${LOG_DIR}/run.log"
    echo "[hint] Check logs at ${NODE_LOG}" | tee -a "${LOG_DIR}/run.log"
    exit 1
  fi
done

echo "[info] Running tests..." | tee -a "${LOG_DIR}/run.log"

cargo test --release -- --nocapture --test-threads=1

echo "[info] Tests completed." | tee -a "${LOG_DIR}/run.log"
