# Deployment Guide - Web3 Destroyer

Autonomous economic exploit detection agent for DeFi protocols.

## Security Notice

**READ-ONLY ANALYSIS**: This tool performs read-only analysis by default. It:
- Fetches contract bytecode from public blockchain
- Simulates transactions on forked state
- Generates PoC files for verification
- Does NOT require or use private keys
- Does NOT sign or broadcast transactions

## Prerequisites

- Rust 1.70+
- An RPC endpoint (e.g., Alchemy, Infura, or a local node)
- Optional: Etherscan API key for source code verification

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DRPC_URL` | Yes | - | RPC endpoint (e.g., https://eth-mainnet.alchemyapi.io/v2/...) |
| `NTFY_TOPIC` | No | web3-destroyer-alerts | ntfy.sh topic for alerts |
| `CHECK_INTERVAL_SECS` | No | 300 | Seconds between monitoring cycles |
| `TARGET_ADDRESSES` | No | (none) | Comma-separated contract addresses to monitor |
| `ETHERSCAN_API_KEY` | No | (none) | Etherscan API key for verification |

## Quick Start

### 1. Build the binary

```bash
cargo build --release
```

### 2. Run single scan

```bash
export DRPC_URL="https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY"
./target/release/web3-destroyer scan 0x1234...5678
```

### 3. Run continuous monitoring

```bash
export DRPC_URL="https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY"
export NTFY_TOPIC="my-alerts"
export TARGET_ADDRESSES="0xContract1,0xContract2"
./target/release/web3-destroyer monitor
```

## Deployment Options

### Option 1: Screen/Tmux (Simplest)

For testing or development:

```bash
# Create a screen session
screen -S web3-destroyer

# Run the agent
export DRPC_URL="https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY"
export TARGET_ADDRESSES="0xA0b86a33E6441C18C541dC6F4f5EEd6e8c8F1E2F"  # Example: Aave V3
./target/release/web3-destroyer monitor

# Detach: Ctrl+A, D
# Reattach: screen -r web3-destroyer
```

### Option 2: Docker

Create a Dockerfile:

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/web3-destroyer /usr/local/bin/
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["web3-destroyer"]
```

Build and run:

```bash
docker build -t web3-destroyer .
docker run -d \
  -e DRPC_URL="https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY" \
  -e NTFY_TOPIC="my-alerts" \
  -e TARGET_ADDRESSES="0xA0b86a33E6441C18C541dC6F4f5EEd6e8c8F1E2F" \
  --name web3-destroyer \
  web3-destroyer monitor
```

### Option 3: Systemd Service (Production)

Create `/etc/systemd/system/web3-destroyer.service`:

```ini
[Unit]
Description=Web3 Destroyer - Economic Exploit Detection
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/web3-destroyer
Environment=DRPC_URL=https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY
Environment=NTFY_TOPIC=web3-destroyer-alerts
Environment=TARGET_ADDRESSES=0xA0b86a33E6441C18C541dC6F4f5EEd6e8c8F1E2F,0x..."
ExecStart=/home/ubuntu/web3-destroyer/target/release/web3-destroyer monitor
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable web3-destroyer
sudo systemctl start web3-destroyer
# Check status
sudo systemctl status web3-destroyer
# View logs
journalctl -u web3-destroyer -f
```

## Notification Setup

The agent uses [ntfy.sh](https://ntfy.sh/) for alerts. This is compatible with NTYY.

### Android/iOS Setup

1. Install the ntfy app
2. Subscribe to your topic (e.g., `web3-destroyer-alerts`)
3. You'll receive push notifications when vulnerabilities are found

### Custom Server

For self-hosted notifications:

```bash
# Use your own ntfy server
export NTFY_SERVER="https://ntfy.mydomain.com"
# The notifier will use: https://ntfy.mydomain.com/{topic}
```

## Monitoring Targets

### Recommended Targets by Category

**Lending Protocols** (high-value targets):
- Aave V3: `0x87870Bca3F3f6335e32cdC0d59b5b5E3C05E5D25`
- Compound V3: `0xA3c788B1a8bD82596B747B112C01B20EF72B91a5`
- Yearn: `0xFC06bACF9B8E8B0d0Eb95547e1aF2e1dAa11fA6F`

**DEXes** (oracle manipulation targets):
- Uniswap V3: `0x1F98431c8aD98523631AE4a59f267346ea31F984`
- Curve: `0x4e3aBDDD83b363B2B61774af29894D17F84eB7Bb`

**Oracles** (critical infrastructure):
- Chainlink: `0xFE270FC1D6d2e67eB07bD99bA8f69E1c826f6a76`
- Uniswap TWAP: `0xE592427A0AEce92De3Edee1F18E0157C05861564`

## Verification & False Positives

The agent uses a two-stage verification pipeline:

1. **Halmos**: Formal verification for mathematical proof
2. **Exploit Synthesis**: Simulation on forked chain state

This significantly reduces false positives. However, always:
- Manually review findings before reporting
- Test PoC on testnet first
- Verify with protocol team (responsible disclosure)

## Troubleshooting

### RPC Errors

If you see RPC errors:
- Check your API key is valid
- Ensure the endpoint supports `eth_getCode` and `eth_call`
- Try a different RPC provider

### No Targets Configured

If `TARGET_ADDRESSES` is empty, the monitor will run but analyze nothing. Set it to at least one contract address.

### Rate Limiting

If you hit rate limits:
- Increase `CHECK_INTERVAL_SECS`
- Use a premium RPC endpoint with higher limits
- Add multiple RPC URLs (future enhancement)

## Updating

```bash
git pull
cargo build --release
sudo systemctl restart web3-destroyer
```

## Support

- GitHub Issues: Report bugs and feature requests
- Always test on testnet before mainnet deployment

---

**DISCLAIMER**: This tool is for educational and defensive security purposes only. Always follow responsible disclosure practices when finding vulnerabilities.