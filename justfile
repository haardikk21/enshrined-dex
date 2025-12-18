# List all available commands
default:
    @just --list

run:
    rm -rf ./dex-reth && cargo run -p reth-node -- node \
        --datadir ./dex-reth \
        --dev \
        --dev.block-time 2s \
        -vvv

run-explorer:
    cd explorer && bun run dev
