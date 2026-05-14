# Economic Exploit Detection Agent - Architecture

## Phase Overview

### Phase 1: Context-First Fuzzing (IMPLEMENTED)
- Oracle-driven fuzzer with grammar-aware input generation
- 40+ DeFi function selectors across 8 protocol types
- Multi-mutation strategies with fitness-based corpus management

### Phase 1.5: Enhanced Symbolic Execution (IMPLEMENTED)
- True Z3 constraint solving for DeFi invariants
- Taint analysis for oracle detection
- Storage slot analysis

### Phase 1.75: Advanced Fuzzing (IMPLEMENTED)
- Economic-guided mutations targeting financial variables
- Profit-based fitness functions (not just coverage)
- Dynamic dictionary evolution from Z3 solutions

### Phase 2: Multi-Contract Analysis (IMPLEMENTED)
- Flash loan attack detection
- Cross-protocol exploit modeling
- Protocol-specific bug hunters for Aave V3 and Uniswap V3

## Depth Levels

### Level 1: Single-Contract Economic Modeling (✅ COMPLETE)
- Basic economic invariants (health factor, constant product, oracle bounds)
- Single transaction exploit modeling
- Profit optimization with EliteZ3Solver
- Economic-guided fuzzing with EliteFuzzer

### Level 2: Multi-Contract Composability (✅ COMPLETE)
- Flash loan attacks (atomic multi-transaction exploits)
- Cross-contract state tracking
- Multi-step transaction sequences (borrow → exploit → repay)

### Level 3: Protocol-Specific Bugs (✅ COMPLETE)
- Aave V3: Health factor miscalculation, liquidation bonus exploits, interest rate manipulation
- Uniswap V3: TWAP manipulation, sandwich attacks, concentrated liquidity arbitrage

## Key Components

### EliteZ3Solver (`elite_z3_solver.rs`)
- Phase 1: Economic constraint modeling for lending, DEX, oracle protocols
- Phase 2: Storage-aware modeling with actual contract storage slot reading
- Phase 3: Multi-step transaction state machine modeling
- Phase 4: Profit optimization using Z3 maximize/minimize tactics
- Phase 5: Precise calldata synthesis from Z3 model output

### EliteFuzzer (`elite_fuzzing_engine.rs`)
- Economic-guided mutations targeting financial variables (balances, prices, rates)
- Profit-based fitness functions (not just coverage)
- Dynamic dictionary evolution from Z3 solutions
- Storage-aware seeding from actual contract values
- Invariant-proximity guidance system
- Bidirectional feedback loop with EliteZ3Solver

### MultiContractAnalyzer (`multi_contract_exploit_analyzer.rs`)
- Flash loan attack detection
- Protocol-specific bug hunters (Aave V3, Uniswap V3)
- TWAP manipulation detection
- Sandwich attack detection
- Concentrated liquidity arbitrage detection
- Quick scan for common DeFi exploits

## Supported Bug Types

### Aave V3 Bugs
- Health factor miscalculation during liquidation
- Liquidation bonus exploit via price manipulation
- Interest rate model manipulation
- Flash loan health factor bypass
- Oracle manipulation (TWAP)

### Uniswap V3 Bugs
- TWAP price manipulation
- Sandwich attack opportunity
- Flash loan arbitrage on concentrated liquidity
- Tick boundary edge cases
- Fee on transfer token exploits

### Generic DeFi Bugs
- Flash loan attacks
- Price oracle manipulation
- Reentrancy attacks
- Cross-protocol composability attacks

## Usage

### Quick Scan
```rust
use crate::agents::multi_contract_exploit_analyzer::MultiContractAnalyzer;

let exploits = MultiContractAnalyzer::quick_scan();
for exploit in exploits {
    println!("Found exploit: {}", exploit.vulnerability_type);
}
```

### Protocol-Specific Analysis
```rust
use crate::agents::multi_contract_exploit_analyzer::{MultiContractAnalyzer, Protocol};

let exploits = MultiContractAnalyzer::analyze_protocol(&Protocol::AaveV3);
```

### Elite Fuzzer
```rust
use crate::agents::elite_fuzzing_engine::EliteFuzzer;
use crate::agents::contract_classifier::ContractArchetype;
use crate::agents::invariant_generator::EconomicInvariant;

let invariants = vec![...];
let fuzzer = EliteFuzzer::new(ContractArchetype::Lending, invariants);
let (mutated, fitness) = fuzzer.fuzz_one(&input, &storage);
```

### Storage Tracer (`storage_tracer.rs`)
- Full-power bytecode execution tracing with revm
- SLOAD/SSTORE tracking with value capture
- External call and created contract detection
- Storage layout analysis and slot classification
- Access control point detection
- Reentrancy pattern identification

### Cross-Protocol Chain (`multi_contract_exploit_analyzer.rs`)
- Petgraph-based multi-hop chain analysis
- Triangular arbitrage detection
- 3+ protocol hop exploit modeling
- Dijkstra/A* pathfinding for profit optimization

### Level 5: Cross-Protocol Chains (✅ ELITE)
- Multi-hop flash loan attack modeling
- Protocol relationship graph with petgraph
- Triangular arbitrage detection
- A* pathfinding for optimal exploit paths

### Level 6: Fork Validation (🚀 IN PROGRESS)
- Real mainnet state replay
- Proof validation against live contracts

## Exit Plan (Current Phase)

### What's Done ✅
- EliteZ3Solver with economic constraint modeling
- EliteFuzzer with bidirectional Z3 feedback
- MultiContractAnalyzer with protocol-specific bug hunters
- Storage tracer with full bytecode analysis
- Cross-protocol chain analyzer with petgraph
- Precision loss modeling in Z3

### What's Next (Next Agent)
1. Complete fork transaction replay integration
2. Add real-time mainnet monitoring
3. Implement Halmos formal verification integration
4. Add bytecode-level invariant extraction
5. Implement upgrade collision detection

## Next Phase Roadmap (Cursed/Zero-Day)

### Phase 3: Cursed/Deep Analysis
1. **Halmos Integration**: Formal verification with a16z Halmos
2. **Bytecode Invariant Extraction**: Dynamic invariant generation from bytecode
3. **Control Flow Analysis**: CFG building for reentrancy/loop detection
4. **Upgrade Collision Detection**: Proxy pattern vulnerability hunting
5. **Real Fork Live Testing**: Continuous mainnet monitoring with transaction replay
6. **Cross-Protocol Chain Validation**: Full 3+ protocol hop testing with real state
7. **Gas/MEV Exploitation**: Automated MEV bot generation

### Target Depth: Level 7 (Zero-Day)
- Formal proofs of exploitability
- Bytecode-only vulnerability detection
- Real-time zero-day discovery
- Automated PoC generation