# Start dev environment (Rust server + UI watcher)
dev port='9000':
    #!/usr/bin/env bash
    set -e
    trap 'kill 0' EXIT
    cd ui && bun run build -- --watch &
    RUST_LOG=debug cargo watch -x 'run -- --data-dir ./data --port {{port}}' &
    wait
