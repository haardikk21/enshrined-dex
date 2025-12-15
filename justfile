# List all available commands
default:
    @just --list

run-bp:
    cd builder-playground && go run main.go cook opstack \
        --external-builder http://host.docker.internal:4444/ \
        --enable-latest-fork 0 \
        --flashblocks \
        --flashblocks-builder ws://host.docker.internal:1111/ws

run-builder:
    cd op-rbuilder && cargo run -p op-rbuilder \
        --bin op-rbuilder -- node \
        --chain $HOME/.playground/devnet/l2-genesis.json \
        --flashblocks.enabled \
        --datadir ~/.playground/devnet/op-rbuilder \
        -vv \
        --http --http.port 2222 \
        --authrpc.addr 0.0.0.0 --authrpc.port 4444 --authrpc.jwtsecret $HOME/.playground/devnet/jwtsecret \
        --port 30333 --disable-discovery \
        --metrics 127.0.0.1:9011 \
        --rollup.builder-secret-key ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
        --trusted-peers enode://79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8@127.0.0.1:30304
