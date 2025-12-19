# List all available commands
default:
    @just --list

run:
    rm -rf ./dex-reth && \
        RUST_LOG=dex=debug,payload_builder=info \
        cargo run -p reth-node -- node \
        --datadir ./dex-reth \
        --dev \
        --dev.block-time 200ms \
        -vvv

run-script:
    cd scripts && bun run index.ts

run-explorer:
    cd explorer && bun run dev
