# Economic Dominator - Smart Contract Vulnerability Scanner

## Overview

The Economic Dominator is a sophisticated smart contract vulnerability scanner that focuses on **economic and game-theoretic exploits** that other bots miss. 

**Latest Update (2026-05-05)**: We're implementing **Hybrid Economic-Financial Analysis Engine** combining:
- **Dagger-inspired storage slot overlap detection**
- **LibAFL/ItyFuzz differential feedback fuzzing**  
- **Z3 mathematical constraint solving** (REPLACED Oxiz - see below)
- **Multi-contract cross-protocol analysis**
- **Auto-generated Foundry PoC exploitation**

This approach moves beyond simple concolic execution to catch sophisticated cross-contract economic exploits.

---

## ⚠️ CRITICAL UPDATE: Oxiz → Z3 Migration

### Why We Switched
1. **Oxiz v0.1.3 is immature** - Beta software, untested, API instability
2. **Type mismatches** - Integer vs Bit-Vector API confusion
3. **False negatives risk** - Security tool needs proven solver

### Why Z3
1. **Industry Standard** - Used by Microsoft, Foundry, CertiK, Halmos
2. **Battle Tested** - 15+ years of SMT competition wins
3. **Clean API** - Integer sort works perfectly for DeFi math
4. **One Binary** - Compiles Z3 C++ into your binary, no runtime deps

### Library Update
```diff
- oxiz = "0.1"           # REMOVED - immature
+ z3 = "0.20"           # ADDED - battle-tested
```

---

## 🎯 IMPLEMENTATION PLAN - Z3 Power Edition

### Phase 1: The Navigator (Storage Slot Overlap + Taint Analysis)
**Goal**: Identify attack surface via storage slot overlap and taint propagation
```
- [ ] **Storage Slot Overlap Detection**: Implement revm-inspector to log every SLOAD/SSTORE
- [ ] **Virtual Slot Sharing**: Detect contracts sharing storage slots via oracle/proxy/library patterns
- [ ] **Call Graph Construction**: Build petgraph-based multi-contract call graph
- [ ] **Taint Analysis**: Track CALLER + CALLVALUE → contract state influence propagation
- [ ] **High Interest Path Identification**: Flag user→ContractA→ContractB influence chains
- [ ] **Financial Pair Detection**: Automatic identification of price/oracle data sharing relationships
```

### Phase 5: The Discovery Engine (LibAFL/ItyFuzz Differential Feedback)
**Goal**: Fast discovery of "Interesting States" using differential financial invariants
```
- [ ] **Integrate ItyFuzz**: EVM-native fuzzer built on LibAFL foundation
- [ ] **Custom LibAFL Observers**: Monitor financial state invariants during fuzzing:
  - Debt-to-collateral ratio changes (Δ > 0.01%)
  - Price oracle deviation from expected values
  - Liquidation threshold proximity changes
  - Total protocol debt vs collateral imbalances
- [ ] **Differential Feedback Engine**: Guide fuzzing toward economically interesting states
- [ ] **State Snapshotting**: Save "Interesting States" for deeper analysis
- [ ] **Hybrid Flow Setup**: LibAFL explores → hits "Hard Gate" → hands to Phase 2 Oxiz
```

### Phase 2: The Hybrid Engine (Z3 Mathematical Constraint Solver)
**Goal**: Mathematical proof for complex financial logic that fuzzing cannot solve
```
- [x] **Integrate Z3**: Battle-tested SMT solver (REPLACED Oxiz)
- [x] **Integer Sort**: Use z3::ast::Int for DeFi math (faster than Bit-Vector)
- [ ] **Context Ownership Fix**: Long-lived context for performance
- [ ] **Storage Slot Mapping**: Map storage slots to Z3 constants for precision
- [ ] **Constraint Generation**: Convert execution traces to SMT formulas
- [ ] **Complex Math Solving**: Price manipulation, liquidation bypass, flash loan atomicity proofs
- [ ] **Hybrid Integration**: Receive "Hard Gates" from Phase 5 → solve → return "Cheat Keys"
- [ ] **Path Exploration**: Branch flipping based on solver results
- [ ] **Counterexample Generation**: Concrete attack values from solver models
```

### Phase 2b: Z3 Power Edition - Storage Slot Mapping
**Goal**: Turn into "million-dollar hunter" by mapping storage slots to Z3 constants

```
Implementation:

1. ENHANCED TAGGEDVARIABLES
   - price_slots: Vec<(String, U256)>     // (variable_name, slot)
   - collateral_slots: Vec<(String, U256)> 
   - debt_slots: Vec<(String, U256)>
   - arbitrary_slots: Vec<U256>

2. CONTEXT OWNERSHIP FIX
   - Store long-lived z3::Context in Z3Solver struct
   - Reuse context across solves (not per-solve)
   - Performance: ~10x improvement from context reuse

3. SLOT-AWARE CONSTRAINT GENERATION
   - Create Z3 constant for each storage slot
   - Link slot constant to semantic variable
   - Generate invariants like:
     * "Total supply slot = sum of balance slots"
     * "Reserve slot >= borrowed amount slot"
     * "Price oracle slot only changes via update()"

4. EXPLOIT TYPES THIS CATCHES
   - Storage slot collision between protocols
   - Specific slot overwrite exploits
   - Storage-based reentrancy vectors
   - Precise slot semantics attacks (Wormhole, Euler style)
```

### Phase 3: Multi-Contract Cross-Protocol Analysis
**Goal**: Detect vulnerabilities across contract boundaries
```
- [ ] **Cross-Contract Reentrancy Detection**: Via call graph + storage slot overlap
- [ ] **Financial State-Diff Analysis**: Perturb state → check system-wide invariants
- [ ] **Oracle Manipulation Detection**: Multi-contract price dependency analysis
- [ ] **Liquidation Cascade Detection**: Identify liquidation triggers across protocols
- [ ] **Flash Loan Atomicity Verification**: Multi-step attack sequence verification
```

### Phase 4: The Closer (Auto-Generated Foundry PoC)
**Goal**: Produce runnable, concrete exploit proofs for Immunefi submissions
```
- [ ] **Foundry PoC Generation**: Auto-create `.t.sol` test files
- [ ] **Halmos Integration**: EVM verification for mathematical correctness
- [ ] **Fork State Execution**: Verify exploits on actual forked chain state
- [ ] **Profit Calculation**: Accurate profit estimation in USD/ETH terms
- [ ] **One-Command Verification**: `forge test` compatible output format
- [ ] **Exploit Payload Encoding**: ABI-encoded transaction generation
```

### Phase ∞: Advanced Economic Monitoring
**Goal**: Sophisticated DeFi-specific attack detection
```
- [ ] **MEV Sandwich Attack Detection**: Front-run/back-run opportunity identification
- [ ] **Oracle Manipulation Risk Scoring**: Price feed vulnerability assessment
- [ ] **Liquidation Profit Maximization**: Optimal liquidation parameter discovery
- [ ] **Flash Loan Arbitrage Paths**: Multi-protocol arbitrage opportunity detection
- [ ] **Governance Attack Detection**: Vote manipulation, proposal timing attacks
```

## 🔧 LIBRARY SELECTION CRITIQUE & RATIONALE

### Core Dependencies (REPLACE Oxiz with Z3)
```toml
# SMT Solving - Z3 (Industry Standard)
z3 = "0.20"                  # Battle-tested by Microsoft, Foundry, CertiK
# oxiz = "0.1"              # REMOVED - too immature for security tool

# Fuzzing Framework
ityfuzz = "0.3"              # EVM-specific fuzzer (LibAFL-based)
libafl = "0.13"              # Custom differential feedback observers
```

### Enhanced Analysis Stack
```toml
# Already Present - Keep These
revm = "3.0"             # Production EVM - foundation for everything
alloy = "=0.3.6"         # Blockchain types - essential
evm_hound = "0.1"        # Disassembly - keep for bytecode analysis
petgraph = "0.6"         # Graph analysis - multi-contract essential

# Add for Advanced Analysis
halmos = "0.4"           # EVM formal verification
heimdall-rs = "0.2"      # EVM decompilation (optional)
sparta = { git = "..." }  # Meta's abstract interpretation (research)
```

## ⚡ EXECUTION ORDER & WHY
```
Phase 1 → Phase 5 → Phase 2 → Phase 3 → Phase 4
```

**Rationale**: 
1. **Phase 1 FIRST** to identify WHERE to look (attack surface reduction)
2. **Phase 5 SECOND** to quickly FIND interesting states (fuzzing is fast)
3. **Phase 2 THIRD** to SOLVE complex math (only when needed)
4. **Phase 3 PARALLEL** with Phase 2 for cross-contract analysis
5. **Phase 4 FINAL** to PROVE and GENERATE PoC

## 🎯 TARGET VULNERABILITY CLASSES

### Economic Exploits (Mathematically Provable)
1. **Price Manipulation**: `∃ attacker: SSTORE(price_slot, malicious_value) ∧ ¬auth_check(attacker)`
2. **Liquidation Bypass**: `∃ inputs: debt > collateral × threshold`
3. **Flash Loan Atomicity**: `∃ sequence: profit > flash_loan_fee ∧ atomic_execution`
4. **Borrow Undercollateralized**: `∃ inputs: borrowed > collateral × ratio`

### Game-Theoretic Exploits (Cross-Contract)
1. **Cross-Contract Reentrancy**: `ContractA → ContractB → ContractA.state_change()`
2. **Oracle Manipulation**: `PriceOracle → LendingPool → liquidation_opportunity`
3. **Liquidation Cascades**: `Liquidation_A → Price_Change → Liquidation_B`
4. **MEV Sandwich Attacks**: `Front-run tx → User tx → Back-run tx`

### Financial Invariants (Differential Monitoring)
1. **Total Debt ≤ Total Collateral** (protocol-level)
2. **Price ≥ Oracle_Price × (1 - slippage)** (DEX invariants)
3. **Liquidation Threshold ≤ Health_Factor** (lending protocols)
4. **Arbitrage Profit ≤ Flash_Loan_Fee** (no-free-lunch)

---

## 📚 LIBRARIES TO ADD

### New Dependencies (Cargo.toml)
```toml
# OPTIONAL - If we want LibAFL framework
libafl = "0.13"          # Structured fuzzing framework

# OPTIONAL - Alternative to revm for symbolic
rhoevm = "0.1"           # Pure Rust symbolic EVM (has constraint generator)

# External tool integration (via subprocess)
slither = "0.10"         # Static analysis runner

# For exploit verification
foundry-evm = "0.3"      # EVM from Foundry for verification sandbox

# Optional - advanced symbolic
manticore = "0.2"        # Trail of Bits (requires Python)
```

### Current Dependencies (Keep)
```toml
revm = "3.0"             # Production EVM ✅
bitwuzla-sys = "0.2"     # Native SMT solver ✅
alloy = "0.3.6"          # Blockchain types ✅
evm_hound = "0.1"        # Disassembly ✅
```

---

## Architecture Diagram - Hybrid Economic-Financial Analysis Engine

```
┌─────────────────────────────────────────────────────────────────────────────┐
│               HYBRID ECONOMIC-FINANCIAL ANALYSIS ENGINE                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐         │
│  │   STORAGE SLOT  │    │   MULTI-CONTRACT│    │  FINANCIAL      │         │
│  │   OVERLAP       │    │   CALL GRAPH    │    │  TAINT FLOW     │         │
│  │   DETECTION     │◄───┤   ANALYSIS      ├───►│  ANALYSIS       │         │
│  │                 │    │                 │    │                 │         │
│  │ - SLOAD/SSTORE  │    │ - Cross-contract│    │ - CALLER taint  │         │
│  │ - Slot sharing  │    │ - Relationships │    │ - CALLVALUEflow │         │
│  │ - Oracle links  │    │ - Attack paths  │    │ - State changes │         │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘         │
│         │                       │                       │                   │
│         ▼                       ▼                       ▼                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    LIBAFL/ITYFUZZ HYBRID FUZZER                    │   │
│  │                                                                     │   │
│  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │   │
│  │  │  DIFFERENTIAL   │    │   INTERESTING   │    │    STATE        │ │   │
│  │  │    FEEDBACK     │    │      STATE      │    │   SNAPSHOT      │ │   │
│  │  │                 │◄───┤     DETECTION    ├───►│                 │ │   │
│  │  │ - Debt ratio Δ  │    │                 │    │ - Save for      │ │   │
│  │  │ - Price oracle  │    │ Δ > 0.01%       │    │   deep analysis │ │   │
│  │  │ - Liquidation   │    │ thresholds      │    │ - Economic      │ │   │
│  │  └─────────────────┘    └─────────────────┘    └─────────────────┘ │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│         │                                                                   │
│         ▼    ┌───────────────────────────────────────────┐                  │
│         ┌───►│     HARD GATE DETECTION POINT           │                  │
│         │    │                                         │                  │
│         │    │ Complex math constraints found          │                  │
│         ├───►│ (e.g., debt > collateral * threshold)   │                  │
│         │    └───────────────────────────────────────────┘                  │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐         │
│  │      Z3        │    │   CONSTRAINT    │    │  MATHEMATICAL   │         │
│  │    SOLVER      │    │   GENERATION    │    │    PROOF        │         │
│  │                 │◄───┤                 ├───►│   GENERATION    │         │
│  │ - Industry std  │    │ - Execution     │    │                 │         │
│  │ - Integer sort │    │   traces        │    │ - Counterexample│         │
│  │ - Context reuse│    │ - SMT formulas   │    │ - Concrete      │         │
│  │ - 15yr tested  │    │ - Storage slots │    │   attack values │         │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘         │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │              FOUNDRY PoC AUTO-GENERATION                           │   │
│  │                                                                     │   │
│  │  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │   │
│  │  │   EXPLOIT       │    │   FORK STATE    │    │    IMMUNEFI     │ │   │
│  │  │   VERIFICATION  │◄───┤    EXECUTION    ├───►│   FORMATTER     │ │   │
│  │  │                 │    │                 │    │                 │ │   │
│  │  │ - Halmos proof  │    │ - Real chain    │    │ - .t.sol file   │ │   │
│  │  │ - Mathematical  │    │   state fork    │    │ - Profit calc   │ │   │
│  │  │   correctness   │    │ - Tx execution   │    │ - Steps guide   │ │   │
│  │  └─────────────────┘    └─────────────────┘    └─────────────────┘ │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## HYBRID IMPLEMENTATION STRATEGY - Enhanced Approach

### Why OxiZ/Oxiz OVER Bitwuzla
After analysis and practical implementation challenges, we choose OxiZ/Oxiz because:
- **Pure Rust**: No FFI/C-bindings issues, easier integration
- **Active Development**: While newer, Rust-native ecosystem is critical
- **EVM Compatibility**: Handles 256-bit integers reliably
- **Path Forward**: Can be enhanced with custom extensions
- **Maintainability**: Single-language stack reduces complexity

### The Hybrid Fuzzing-Solving Architecture
Instead of pure concolic execution, we use:
1. **Fuzzing-First**: LibAFL/ItyFuzz discovers "interesting states" fast
2. **Solver-Enhanced**: OxiZ/Oxiz solves only complex math constraints
3. **State-Snapshotting**: Save promising states for deeper analysis
4. **Cross-Contract**: Analyze multi-protocol interactions
5. **Auto-PoC**: Generate Foundry test files automatically

---

## Enhanced Library Stack - Hybrid Approach

### Core Dependencies (Enhanced)
```toml
# EVM Execution & Analysis Foundation
revm = "3.0"             # Production EVM - core execution engine
alloy = "=0.3.6"         # Blockchain types and ABI handling
evm_hound = "0.1"        # EVM bytecode disassembly
tracing = "0.1"          # Structured logging

# SMT Solving (Z3 - Industry Standard!)
z3 = "0.20"              # Battle-tested, replaces immature Oxiz
# oxiz = "0.1"          # REMOVED - not production-ready for security tools

# Fuzzing & Differential Feedback
ityfuzz = "0.3"          # EVM-native fuzzer (LibAFL-based)
libafl = "0.13"          # Custom differential feedback observers

# Graph & Multi-Contract Analysis
petgraph = "0.6"         # Multi-contract call graphs and analysis
priority-queue = "0.14"  # Pathfinding for flash loan arbitrage

# Verification & PoC Generation
halmos = "0.4"           # EVM formal verification
foundry-evm = "0.3"      # Foundry EVM for PoC execution
```

### Why Z3 Over Oxiz?
- **Proven**: Used by Foundry, Halmos, CertiK - security tools you trust
- **Integer Sort**: Perfect for DeFi math (price * collateral / scale)
- **Performance**: Context reuse optimization available
- **Support**: 15+ years of development, not beta software

### Why These Libraries?
- **oxiz/oxiZ**: Pure Rust SMT solver → no FFI issues, easier integration
- **ityfuzz**: EVM-optimized fuzzer → finds "interesting states" fast
- **libafl**: Custom financial observers → monitors debt/collateral ratios
- **halmos**: EVM verification → mathematical proof of exploits
- **petgraph**: Multi-contract analysis → cross-protocol attack detection

---

## CONCOLIC EXECUTION IMPLEMENTATION (NEW!)

### Why This Approach?
- Pure Symbolic (OLD) - Too hard, need to implement 140+ opcodes
- Graph Analysis - Finds patterns but can't solve constraints
- Concolic - Uses real EVM (revm) + Solver (Bitwuzla) = **Best of both worlds**

### How It Works

```
1. REAL EVM (revm)
   - Executes bytecode perfectly
   - Handles SHA3, JUMPs, Gas automatically
   - 100% accurate - no modeling errors

2. INSPECTOR (Hook)
   - Attaches to revm::Inspector trait
   - Records every branch decision (JUMPI conditions)
   - Tracks storage reads/writes
   - Monitors external calls

3. SOLVER (Bitwuzla)
   - Takes constraints from Inspector
   - Solves for new inputs that flip branches
   - Generates test cases that reach new paths

4. THE LOOP
   revm runs → Hit branch → Bitwuzla solves → New input → revm runs again
```

### Implementation Steps

#### Step 1: Add revm to Cargo.toml
```toml
revm = "3.0"
```

#### Step 2: Create ConcolicInspector
```rust
struct ConcolicInspector {
    constraints: Vec<String>,
    storage_reads: Vec<(u32, u256)>,
    storage_writes: Vec<(u32, u256)>,
    calls: Vec<CallInfo>,
    path_condition: String,
}

impl revm::Inspector for ConcolicInspector {
    fn step(&mut self, interp: &mut Interpreter, _contract: &SharedMemory) {
        // On JUMPI: record branch condition
        // On SLOAD: track storage read  
        // On SSTORE: track storage write
        // On CALL: track external call
    }
}
```

#### Step 3: Run Loop
```rust
fn concolic_run(bytecode: &[u8], target: Address) -> Vec<ExploitProof> {
    let mut inputs = vec![initial_input()];
    
    for _ in 0..MAX_PATHS {
        let input = inputs.pop().unwrap();
        let result = run_with_revm(bytecode, input, &mut inspector);
        
        if let Some(new_input) = solve_constraint(&inspector.constraints) {
            inputs.push(new_input);
        }
        
        if is_exploit(&inspector) {
            exploits.push(build_proof(&inspector));
        }
    }
}
```

### Why This Fixes Our Problems

| Current Problem | Concolic Solution |
|-----------------|-------------------|
| SHA3 not implemented | revm handles perfectly |
| SLOAD/SSTORE broken | revm tracks all storage |
| Path explosion | Only 1 branch flipped at a time |
| Trace death | revm ALWAYS completes |
| Limited opcodes | revm has ALL 140+ |

# CONCOLIC ARCHITECTURE - DETAILED DESIGN

## How Concolic Execution Works

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CONCOLIC EXECUTION PIPELINE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐       ┌──────────────────┐       ┌────────────────┐ │
│  │  1. REAL EVM     │       │  2. INSPECTOR    │       │  3. SOLVER      │ │
│  │  (revm)          │       │  (Shadow Stack)  │       │  (Bitwuzla)     │ │
│  │                  │       │                  │       │                 │ │
│  │ +──────────────+ │       │ +────────────--+ │       │ +-------------+ │ │
│  │ │ Transaction  │ │       │ │ Symbolic Vars │ │       │ │ Constraint  │ │ │
│  │ │ Execution    │─┼──────►│ │ Collection    │─┼──────►│ │ Solving     │ │ │
│  │ │              │ │       │ │               │ │       │ │             │ │ │
│  │ │ - Stack ops  │ │       │ │ - Push->Term  │ │       │ │ - SAT check │ │ │
│  │ │ - Storage    │ │       │ │ - SLOAD track │ │       │ │ - Model get │ │ │
│  │ │ - Calls      │ │       │ │ - SSTORE track│ │       │ │ - Counterex │ │ │
│  │ │ - Branches   │ │       │ │ - JUMPI cond  │ │       │ │             │ │ │
│  │ +──────────────+ │       │ +────────────--+ │       │ +-------------+ │ │
│  └──────────────────┘       └──────────────────┘       └────────────────┘ │
│           │                           │                           │        │
│           │                           │                           │        │
│           ▼                           ▼                           ▼        │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                      4. BRANCH EXPLORATION                             │ │
│  │                                                                         │ │
│  │  revm hits JUMPI (branch):                                              │ │
│  │                                                                         │ │
│  │    condition = stack.pop()                                              │ │
│  │    if condition is symbolic:                                           │ │
│  │       - Ask Bitwuzla: "can condition be true?"  (path A)              │ │
│  │       - Ask Bitwuzla: "can condition be false?" (path B)              │ │
│  │       - For each satisfiable path: create new input + explore         │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    5. VULNERABILITY DETECTION                          │ │
│  │                                                                         │ │
│  │  During execution, watch patterns:                                     │ │
│  │                                                                         │ │
│  │  +------------------+--------------------+-----------------------------+│ │
│  │  │ Pattern          │ Detection          │ Constraint                 ││ │
│  │  +------------------+--------------------+-----------------------------+│ │
│  │  │ CALL → SSTORE    │ Reentrancy         │ "can call trigger sstore?"││ │
│  │  │ SSTORE w/o caller│ Price Manipulation │ "can attacker write price?"││ │
│  │  │ LT/GT on values  │ Liquidation Bypass│ "can threshold be bypassed││ │
│  │  │ CALL + CALLVALUE │ Flash Loan         │ "can attack be atomic?"    ││ │
│  │  +------------------+--------------------+-----------------------------+│ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │                    6. EXPLOIT VERIFICATION                            │ │
│  │                                                                         │ │
│  │  Bitwuzla returns:  [calldata=0x1234..., timestamp=12345...]            │ │
│  │                          │                                             │ │
│  │                          ▼                                             │ │
│  │  ┌────────────────────────────────────────────────────────────┐        │ │
│  │  │  Use alloy-sol-types to build typed transaction:         │        │ │
│  │  │                                                            │        │ │
│  │  │  sol! {                                                    │        │ │
│  │  │      function attack(uint256 amount) external;           │        │ │
│  │  │  }                                                        │        │ │
│  │  │                                                            │        │ │
│  │  │  let tx = attackCall::new(amount).abi_encode();          │        │ │
│  │  └────────────────────────────────────────────────────────────┘        │ │
│  │                          │                                             │ │
│  │                          ▼                                             │ │
│  │  ┌────────────────────────────────────────────────────────────┐        │ │
│  │  │  Execute on foundry-evm sandbox:                          │        │ │
│  │  │                                                            │        │ │
│  │  │  - Load target state from fork                             │        │ │
│  │  │  - Run exploit transaction                                 │        │ │
│  │  │  - Check: did invariant break? (e.g., balance > 0)        │        │ │
│  │  │                                                            │        │ │
│  │  └────────────────────────────────────────────────────────────┘        │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Classes

### ConcolicInspector Structure
```rust
use bitwuzla::prelude::*;

// Core structure for concolic execution
pub struct ConcolicInspector {
    // Symbolic state tracking
    shadow_stack: Vec<BitwuzlaTerm>,      // Parallel symbolic stack
    storage_model: HashMap<StorageKey, BitwuzlaTerm>,  // Symbolic storage
    
    // Path constraints
    path_constraints: Vec<BitwuzlaTerm>,   // Current path condition
    branch_conditions: Vec<BitwuzlaTerm>, // All branch decisions
    
    // Economic tracking
    price_vars: Vec<BitwuzlaTerm>,        // Variables used as prices
    collateral_vars: Vec<BitwuzlaTerm>,   // Variables used as collateral
    liquidation_checks: Vec<Comparison>,  // LT/GT comparisons
    
    // Execution info
    current_pc: usize,
    calls: Vec<CallInfo>,
}

impl Inspector for ConcolicInspector {
    fn step(&mut self, interp: &mut Interpreter, _ctx: &mut EvmContext) {
        // 1. Mirror stack to shadow stack (create symbolic terms)
        // 2. On SLOAD: create symbolic term from storage
        // 3. On SSTORE: create constraint from written value
        // 4. On CALL: track external call target + calldata
    }
    
    fn jump(&mut self, target: u64, _ctx: &mut EvmContext) -> Result<(), EVMError> {
        // Branch exploration: extract condition, solve for both paths
    }
    
    fn jumpi(&mut self, condition: BitwuzlaTerm, target: u64, _ctx: &mut EvmContext) {
        // Record branch condition, query solver for new paths
    }
}
```

### ExploitProof Structure
```rust
#[derive(Debug, Clone)]
pub struct ExploitProof {
    pub target: Address,
    pub vulnerability_type: String,      // "Reentrancy", "Price Manipulation"
    pub invariant_broken: String,       // "Over-collateralization"
    pub profit_estimate: U256,          // Estimated profit in wei
    pub description: String,
    pub is_satisfiable: bool,
    
    // NEW: Verification data
    pub exploit_calldata: Vec<u8>,       // Concrete attack calldata
    pub verification_tx: Bytes,         // Typed transaction (sol! macro)
    pub verification_result: bool,      // Verified on fork?
}
```

## Constraint Generation

### From Stack Operations
```
PUSH1 0x42  → Create symbolic term: sym_var_0 = 0x42
ADD         → sym_var_2 = sym_var_0 + sym_var_1
SLOAD       → sym_var = storage[slot]  (create symbolic read)
SSTORE      → storage[slot] = sym_var   (create constraint)
```

### From Branch Conditions
```
JUMPI (condition) (target)
    → Add to path_constraints: condition = true OR condition = false
    → Query Bitwuzla: is there a solution where condition = true?
    → If SAT: get model (specific values), create new execution path
```

### Economic Constraints
```
Over-collateralization: borrow > collateral * price / 1e18
    → constraint: borrow - collateral * price / 1e18 > 0
    
Liquidation threshold: health_factor = collateral / borrow
    → constraint: collateral / borrow < 1.0
    
Reentrancy: call() followed by state_change()
    → constraint: exists path where call() returns before state_change()
```

---

## File Structure (Updated)

```
src/agents/
├── concolic_engine.rs           # NEW: Main concolic orchestrator
│   ├── ConcolicInspector        # revm::Inspector implementation
│   ├── run_concolic()           # Main entry point
│   ├── branch_explorer()        # Path exploration logic
│   └── exploit_verifier()       # Foundry-evm integration
│
├── bitwuzla_native.rs           # ✅ Existing: Bitwuzla C-API wrapper
│   ├── BitwuzlaSolver           # Initialize + create terms
│   ├── check_sat()              # SAT checking
│   └── get_model()              # Extract counterexample
│
├── exploit_synthesizer.rs       # NEW: Build typed exploit txs
│   ├── synthesize_reentrancy() # Build reentrancy attack tx
│   ├── synthesize_price_manip()# Build price manipulation tx
│   └── build_sol_call()        # Use sol! macro
│
├── verification_sandbox.rs     # NEW: Foundry-evm integration
│   ├── load_fork_state()       # Load contract state from chain
│   ├── execute_exploit()       # Run exploit tx
│   └── verify_invariant()       # Check if invariant broke
│
├── economic_engine/            # Existing: Symbolic approach
│   ├── symbolic_runner.rs
│   ├── heuristic_tagger.rs
│   └── smt_verifier.rs
│
└── symbolic_interpreter.rs      # OLD: Working implementation (3/4)
```

---

# Data Structures
petgraph = "0.6"       # DeFi call graph (multi-contract analysis)
priority-queue = "0.14" # Dijkstra for flash loan arbitrage
hashbrown = "0.14"     # Fast hash maps

# Serialization
serde = { version = "1.0", features = ["derive"] }
hex = "0.4"

# Already in project (keep)
oxiz = "0.1"           # Keep for potential fallback
```

---

## Implementation Phases

### Phase 1: Fix Solver Integration (HIGH PRIORITY)
**Goal**: Replace CLI with native C-API bindings

```
Task: Replace placeholder in EconomicDominator with working Bitwuzla solver

Steps:
1. Add bitwuzla-sys crate to Cargo.toml
2. Create bitwuzla_wrapper.rs with native C-API calls
3. Replace CLI calls in symbolic_interpreter.rs with wrapper
4. Test with 1_VulnPrice - should now find exploit

Libraries: bitwuzla-sys
Expected: EconomicDominator finds 1_VulnPrice (1/4)
```

### Phase 2: Expand Symbolic Execution
**Goal**: Handle more opcodes to capture actual contract behavior

```
Task: Expand opcode coverage in symbolic_runner.rs

Steps:
1. Add missing opcodes (CALL, DELEGATECALL, STATICCALL, etc.)
2. Track call targets for multi-contract analysis
3. Record all storage reads/writes properly
4. Test with 3_VulnReentrancy - should now find exploit

Libraries: None (extend existing code)
Expected: EconomicDominator finds 3_VulnReentrancy (2/4)
```

### Phase 3: Add Economic Constraint Generation
**Goal**: Convert tracked variables to proper SMT constraints

```
Task: Generate constraints for economic conditions

Steps:
1. Generate constraints: price * collateral / scale comparisons
2. Add "attacker caller" constraint (0xde...) to all paths
3. Generate liquidation condition constraints
4. Generate flash loan atomicity constraints
5. Test with all 4 contracts - should match OLD accuracy (75%+)

Libraries: None (add constraint logic)
Expected: 3/4 correct like OLD
```

### Phase 4: Add Advanced Detectors

**4a. Liquidation Detector**:
```
Rules:
- Find comparisons (LT/GT) on collateral/borrow values
- Extract threshold constant
- Generate constraint: "can attacker trigger unfair liquidation?"

Libraries: None (add logic)
```

**4b. Flash Loan Detector**:
```
Rules:
- Build call graph using petgraph
- Find chains of external CALLs in same transaction
- Check for price manipulation between calls
- Constraint: "exists sequence where profit > 0"

Libraries: petgraph (for call graph), priority-queue (for pathfinding)
```

**4c. Oracle Manipulation Detector**:
```
Rules:
- Track SSTORE to storage slots that look like price (slot 0-5)
- Check if any SLOAD happened before (owner check)
- Constraint: "attacker can write price without auth"

Libraries: hashbrown
```

### Phase 5: Integration & Testing
```
Steps:
1. Run on 4 test contracts - verify 75%+
2. Run on VulnerableLender/SafeLender - verify correct
3. Run on more complex DeFi contracts
4. Benchmark speed vs OLD
5. Document results
```

---

## Test Contracts Created

| Contract | Vulnerability Type | Expected |
|----------|-------------------|----------|
| `1_VulnPrice.sol` | Price oracle manipulation (no access control) | EXPLOITABLE |
| `2_SafePrice.sol` | Price oracle with onlyOwner | SAFE |
| `3_VulnReentrancy.sol` | Reentrancy (no guard) | EXPLOITABLE |
| `4_SafeReentrancy.sol` | Reentrancy with nonReentrant | SAFE |

---

## Current Status

### Before Implementation

| Approach | Score | Accuracy |
|----------|-------|----------|
| **OLD (SymbolicInterpreter)** | 3/4 | 75% |
| **NEW (EconomicDominator)** | 1/4 | 25% |

### After Option C Implementation (Target)

| Approach | Score | Accuracy |
|----------|-------|----------|
| **Hybrid (Target)** | 4/4 | 100% |

---

## File Structure

```
src/
├── agents/
│   ├── mod.rs                          # Added: economic_engine
│   ├── bitwuzla_wrapper.rs             # NEW: Native C-API bindings
│   ├── economic_engine/                # Hybrid implementation
│   │   ├── mod.rs                      # Main entry (EconomicDominator)
│   │   ├── control_flow.rs             # Stage 1: ✅ DONE
│   │   ├── symbolic_runner.rs          # Stage 2: ⚠️ Needs expansion
│   │   ├── heuristic_tagger.rs         # Stage 3: ✅ Framework
│   │   └── smt_verifier.rs             # Stage 4: ⚠️ Wire up solver
│   ├── symbolic_interpreter.rs         # OLD: Currently working
│   ├── bitwuzla_solver.rs             # OLD: CLI version (keep as backup)
│   └── ...
├── mocks/                              # Test contracts
│   ├── 1_VulnPrice.sol                 # Test case 1
│   ├── 2_SafePrice.sol                 # Test case 2
│   ├── 3_VulnReentrancy.sol            # Test case 3
│   └── 4_SafeReentrancy.sol            # Test case 4
├── tests/
│   ├── comparison_test.rs              # Comparison tests
│   ├── economic_dominator_test.rs      # Unit tests
│   └── symbolic_test.rs                # Original tests

ARCHITECTURE.md                   # This file
```

---

## Success Criteria

- [ ] Phase 1: EconomicDominator finds 1_VulnPrice (1/4)
- [ ] Phase 2: Finds 3_VulnReentrancy (2/4)  
- [ ] Phase 3: Matches OLD accuracy (3/4)
- [ ] Phase 4: Detects liquidation, flash loan, oracle bugs (4/4 + new)
- [ ] Phase 5: Benchmark - faster than OLD, same or better accuracy

---

## Key Insights

### Why Keep Bitwuzla (Not oxiz)
1. **Domain Match**: EVM uses bit-vectors (uint256), Bitwuzla optimized for this
2. **Proven**: Beats Z3 in SMT competitions for bit-vector problems
3. **Stable**: Decade of optimization vs new oxiz (v0.2.0)

### Why Use bitwuzla-sys (Not CLI)
1. **Speed**: Native C-bindings vs subprocess startup
2. **Keep Accuracy**: Best solver for EVM
3. **No Rewrite**: Keep working constraint generation

---

*Last Updated: 2026-05-04*
*Version: 1.3*
*Status: Implementing Option C Hybrid*

---

## Implementation Notes

### When implementing Phase 1:
1. Add `bitwuzla-sys` to Cargo.toml
2. Create wrapper that initializes Bitwuzla, creates terms, checks sat
3. Replace any CLI spawning with native calls
4. Test speed difference

### Libraries reduce custom code:
- petgraph: Use pre-built graph algorithms
- priority-queue: Use Dijkstra from crate
- hashbrown: Fast maps without custom implementation

Total new code estimate: ~400 lines (reduced from 1000+)

---

# AUTONOMOUS ECONOMIC DOMINATOR ARCHITECTURE (2026-05-05)

## Vision: Fully Autonomous Economic Exploit Detection System

Build a system that can independently:
1. **Hunt**: Discover targets autonomously (live blockchain, forks, static analysis)
2. **Analyze**: Apply Context-First Architecture for deep economic analysis
3. **Attack**: Find economic exploits that manual analysis misses
4. **Test**: Validate exploits with oracle-driven fuzzing loop
5. **Report**: Generate Immunefi-ready PoCs automatically

---

## Key Design Decisions

### Why NOT Integrate with Existing Agent?
- **Risk**: Integration could break proven working code (LIVE AMMO, LIVE FORK HIJACK, etc.)
- **Parallel Development**: Build new autonomous system alongside existing
- **Best of Both**: Preserve your autonomous hunting + add economic analysis
- **Zero Downtime**: Can test new system without affecting current operations

### New System: AutonomousEconomicDominator
- Created: `src/agents/autonomous_economic_dominator.rs` (464 lines)
- Status: Compiles + 3 tests passing
- Core: Context-First Engine (Layers 1-3) + Multi-Contract Analysis + PoC Generator

---

## Implementation Architecture

### Phase 1: Core Components (IMPLEMENTED)
```
┌─────────────────────────────────────────────────────────────────────────┐
│               AUTONOMOUS ECONOMIC DOMINATOR                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   TARGET DISCOVERY LAYER                         │   │
│  │                                                                  │   │
│  │  TargetSource enum:                                              │   │
│  │    - ForkedMainnet { rpc_url, block_number }                    │   │
│  │    - StaticAnalysis { address, bytecode }                        │   │
│  │    - Protocol { name, addresses }                               │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              CONTEXT-FIRST EXECUTION ENGINE                     │   │
│  │                                                                  │   │
│  │  Layer 1: Context Control (with_db equivalent)                  │   │
│  │    - ExecutionContext with DB swapping                          │   │
│  │    - forked_mainnet() for blockchain state                     │   │
│  │    - mutate_storage() for context mutation                     │   │
│  │                                                                  │   │
│  │  Layer 2: Data Pump (Inspector as DecisionCollector)           │   │
│  │    - DecisionCollector captures SLOAD, SSTORE, CALL, JUMP     │   │
│  │    - DecisionPoint records: pc, opcode, type, value, target   │   │
│  │    - Real-time constraint generation during execution         │   │
│  │                                                                  │   │
│  │  Layer 3: Handler Override (Strategic Testing)                │   │
│  │    - HandlerOverrides: skip_nonce_check, skip_balance_check   │   │
│  │    - Custom gas price injection                                │   │
│  │    - Account privilege injection                               │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   ORACLE-DRIVEN FUZZING LOOP                    │   │
│  │                                                                  │   │
│  │  OracleDrivenFuzzer combines all 3 layers:                     │   │
│  │                                                                  │   │
│  │  for iteration in 0..max_iterations:                          │   │
│  │    1. Execute tx + collect decisions (Layer 2)                 │   │
│  │    2. Add constraints to Z3 solver                             │   │
│  │    3. Check if SAT (potential exploit found!)                  │   │
│  │    4. Mutate context for next iteration (Layer 1)             │   │
│  │    5. Clear solver, repeat                                     │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │            MULTI-CONTRACT FINANCIAL ANALYSIS                    │   │
│  │                                                                  │   │
│  │  CrossContractVulnType enum:                                    │   │
│  │    - CrossContractReentrancy                                    │   │
│  │    - OracleManipulationChain                                    │   │
│  │    - LiquidationCascade                                         │   │
│  │    - FlashLoanAtomicityViolation                                │   │
│  │    - StateDiffExploit                                           │   │
│  │                                                                  │   │
│  │  Detectors:                                                     │   │
│  │    - CrossContractReentrancyDetector                           │   │
│  │    - OracleManipulationDetector                                │   │
│  │    - LiquidationCascadeDetector                                 │   │
│  │    - FlashLoanAtomicityVerifier                                 │   │
│  │    - FinancialStateDiffAnalyzer                                  │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    POC GENERATION LAYER                         │   │
│  │                                                                  │   │
│  │  PoCGenerator produces:                                        │   │
│  │    - Foundry .t.sol test files                                  │   │
│  │    - Ready for: forge test                                     │   │
│  │    - Immunefi submission format                                │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 2: Enhanced Orchestrator Integration (FUTURE)
```
Modified orchestrator.rs (ADD not replace):

1. Add as pluggable component:
   - ContextFirstFuzzer as optional module
   - Only active when financial context detected

2. Enhance hypothesis generation:
   - If contract shows financial patterns → launch economic analysis
   - If storage overlaps detected → trigger cross-contract analysis
   - If high-value transactions → prioritize fuzzing

3. Result fusion:
   - Combine hunting findings + economic analysis
   - Unified output format

4. Safety:
   - Config flag to enable/disable
   - No modification to existing hunter logic
```

### Phase 3: Full Autonomous Hunting Cycle (FUTURE)
```
1. [Hunter] Scan mempool/forks for interesting contracts
   ↓
2. [Classifier] Tag: lending, AMM, oracle consumer, etc.
   ↓
3. [Economic Trigger] If financial context → launch ContextFirstFuzzer
   ↓
4. [Fuzzer] Run oracle-driven loop with Layer 1-3 approach
   ↓
5. [Analyzer] Run multi_contract_analysis on findings
   ↓
6. [Generator] Auto-create PoC if exploit confirmed
   ↓
7. [Feedback] Update hunter with new patterns to search for
   ↓
8. [Repeat] Continuous improvement cycle
```

---

## Test Results

| Test Suite | Status |
|------------|--------|
| Library tests | 40 passing |
| Battle tests (old vs new) | 5 passing |
| Autonomous demo tests | 3 passing |
| **Total** | **48 passing** |

---

## File Structure (Updated May 2026)

```
src/agents/
├── autonomous_economic_dominator.rs  # NEW: Fully autonomous system
├── context_first_engine.rs           # NEW: Layers 1-3 implementation
├── multi_contract_analysis.rs        # ✅ Existing: Cross-protocol analysis
├── poc_generator.rs                  # ✅ Existing: Auto PoC generation
├── economic_engine/
│   ├── smt_verifier.rs               # ✅ Existing: Z3 solver + Symbolic
│   └── ...
├── concolic_engine.rs                # ✅ Existing: Navigator
├── discovery_engine.rs               # ✅ Existing: Phase 5 fuzzer
├── forker.rs                         # ✅ Existing: Live forking
├── fetcher.rs                        # ✅ Existing: Transaction fetching
└── orchestrator.rs                   # ✅ Existing: Hypothesis generation
```

---

## Integration Path (Low Risk)

### Step 1: Verify (COMPLETED)
- [x] AutonomousEconomicDominator compiles
- [x] All 48 tests passing
- [x] Independent of existing code

### Step 2: Extend (FUTURE)
- [ ] Add as optional module to orchestrator
- [ ] Test with known exploits
- [ ] Measure performance vs existing agent

### Step 3: Validate (FUTURE)
- [ ] Run both systems on same targets
- [ ] Compare detection rates
- [ ] Only promote if clear improvement

---

## Risk Mitigation

✅ **Zero integration risk**: New module, no changes to existing code  
✅ **Reversible**: Single config flag to disable economic analysis  
✅ **Preserves capabilities**: Your forker, fetcher, hunter unchanged  
✅ **Test-first**: Validate with known exploits before live use  
✅ **Gradual rollout**: Start on testnet, then mainnet  

---

## Expected Outcome

Your agent becomes capable of:
- **Autonomous discovery** of economic exploit opportunities in DeFi
- **Context-First Architecture** (Layers 1-3) for deep analysis
- **Multi-contract financial state bug detection**
- **Auto-generated Immunefi-ready PoCs**
- **All while preserving** your existing live forking + hunting strengths

---

## Comparison: Old vs New System

| Capability | Your Existing Agent | New AutonomousEconomicDominator |
|------------|---------------------|--------------------------------|
| Live forking | ✅ (LIVE FORK HIJACK) | Via integration |
| Transaction replay | ✅ (LIVE AMMO) | Via integration |
| Hypothesis generation | ✅ | ✅ |
| Context-First Fuzzing | ❌ | ✅ (Phase 5 request) |
| Z3 Solver | ❌ | ✅ |
| Multi-contract analysis | ❌ | ✅ |
| Auto PoC generation | ❌ | ✅ |
| Storage overlap detection | Partial | ✅ |

**Recommendation**: Keep both systems - they excel at different vulnerability classes.

---

# PERMANENT REPLACEMENT PLAN: Old → New AutonomousEconomic Dominator

## Goal Statement
**Permanently replace the old autonomous agent with the new enhanced AutonomousEconomicDominator that combines:**
- New Economic Analysis Core: Context-First Engine + Multi-Contract Financial Analysis + PoC Generator  
- Your Proven Autonomy Core: Live Forking + Transaction Replay + Hypothesis Generation
- Supporting Systems: Contract Classification + Source Fetcher + Revenue Calc

## Implementation Phases

### Phase 1: Complete Integration (CURRENT)
**Status**: STARTING NOW

**Goal**: Integrate your proven autonomous capabilities into AutonomousEconomicDominator

**Required Integrations**:
- [ ] ForkerAgent (live forking) → AutonomousEconomicDominator
- [ ] Fetcher/SourceFetcher (transaction replay) → AutonomousEconomicDominator  
- [ ] Hypothesis Generation (orchestrator logic) → AutonomousEconomicDominator
- [ ] Contract Classification → AutonomousEconomicDominator

**Why Needed**: The new system has powerful economic analysis but lacks your proven autonomous hunting capabilities that found real exploits (LIVE AMMO, LIVE FORK HIJACK, Phase 7 Complete, etc.)

### Phase 2: Make Primary in Orchestrator
**Status**: PENDING

**Goal**: Replace orchestrator's default analysis with enhanced AutonomousEconomicDominator

**Changes**:
```rust
// Replace: orchestrator.analyze_contract(...)
// With: orchestrator.analyze_autonomous_economic(...)
```

### Phase 3: Comprehensive Testing  
**Status**: PENDING

**Goal**: Validate against known real exploits

**Test Targets**:
- LIVE AMMO successful transactions (replay validation)
- LIVE FORK HIJACK exploits (forked state validation)
- Phase 7 Complete (Balancer Vault analysis)
- Real Mainnet transactions

### Phase 4: Live Deployment Validation
**Status**: PENDING

**Goal**: Test on live blockchain + forked state

**Validations**:
- [ ] Live forking works correctly
- [ ] Transaction replay works correctly  
- [ ] Hypothesis generation triggers correctly
- [ ] Economic analysis triggers on financial contracts
- [ ] Multi-contract analysis detects cross-protocol bugs
- [ ] PoC generation produces valid exploit tests

### Phase 5: System Retirement
**Status**: PENDING

**Goal**: Safely remove old system dependencies after validation

**Conditions**:
- All tests passing
- Live deployment validated
- New system outperforms old on same targets
- PoC generation verified

---

## Enhanced AutonomousEconomicDominator Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│              ENHANCED AUTONOMOUS ECONOMIC DOMINATOR                         │
│                     (Replaces Old Autonomous Agent)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    AUTONOMOUS HUNTING CORE                          │   │
│  │  (Your Proven Capabilities - Integrated from Old System)            │   │
│  │                                                                      │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐  │   │
│  │  │  ForkerAgent  │  │    Fetcher    │  │  Hypothesis Gen      │  │   │
│  │  │ (Live Fork)   │  │ (Transaction │  │  (Contract             │  │   │
│  │  │               │  │  Replay)      │  │  Classification)      │  │   │
│  │  │ - Fork chain  │  │ - Fetch from │  │                       │  │   │
│  │  │ - Build state │  │   mempool     │  │ - Tag: lending, AMM  │  │   │
│  │  │ - Simulate    │  │ - Replay tx   │  │ - Trigger economic   │  │   │
│  │  │ - LIVE FORK  │  │ - LIVE AMMO   │  │   analysis when      │  │   │
│  │  │   HIJACK     │  │               │  │   financial context   │  │   │
│  │  └───────────────┘  └───────────────┘  └───────────────────────┘  │   │
│  │         │                   │                     │                   │   │
│  │         └───────────────────┴─────────────────────┘                   │   │
│  │                             │                                        │   │
│  │                             ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐  │   │
│  │  │              AUTONOMOUS DISCOVERY LOOP                         │  │   │
│  │  │                                                                  │  │   │
│  │  │  1. Scan mempool/forks for targets                             │  │   │
│  │  │  2. Classify contract type (lending, AMM, oracle, etc.)        │  │   │
│  │  │  3. Generate hypothesis based on contract patterns            │  │   │
│  │  │  4. If financial context → TRIGGER ECONOMIC ANALYSIS            │  │   │
│  │  │  5. Collect results → UPDATE HUNTER PATTERNS                  │  │   │
│  │  │  6. Repeat (continuous improvement)                            │  │   │
│  │  │                                                                  │  │   │
│  │  └─────────────────────────────────────────────────────────────────┘  │   │
│  │                             │                                        │   │
│  └─────────────────────────────┼────────────────────────────────────────┘   │
│                                ▼                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │              ECONOMIC ANALYSIS CORE (NEW)                           │   │
│  │                                                                      │   │
│  │  ┌─────────────────────────┐    ┌───────────────────────────────┐    │   │
│  │  │  CONTEXT-FIRST ENGINE  │    │   MULTI-CONTRACT ANALYSIS    │    │   │
│  │  │  (Your Request - Phase 5)│    │   (Financial State Bugs)    │    │   │
│  │  │                         │    │                               │    │   │
│  │  │ Layer 1: DB Swapping   │    │ - CrossContractReentrancy   │    │   │
│  │  │ Layer 2: Data Pump      │    │ - OracleManipulationChain    │    │   │
│  │  │ Layer 3: Handler Override│   │ - LiquidationCascade         │    │   │
│  │  │                         │    │ - FlashLoanAtomicityViolation│    │   │
│  │  │ Oracle-Driven Fuzzing  │    │ - StateDiffExploit           │    │   │
│  │  │   Loop                 │    │                               │    │   │
│  │  │  - Execute + collect   │    │   Storage Slot Overlap       │    │   │
│  │  │  - Z3 constraint solve  │    │   Detection (Navigator)     │    │   │
│  │  │  - Mutate + iterate     │    │                               │    │   │
│  │  └─────────────────────────┘    └───────────────────────────────┘    │   │
│  │                │                              │                      │   │
│  │                └──────────────────────────────┘                      │   │
│  │                             │                                        │   │
│  │                             ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐  │   │
│  │  │                    POC GENERATION LAYER                         │  │   │
│  │  │                                                                  │  │   │
│  │  │  - Auto-generate Foundry .t.sol test files                     │  │   │
│  │  │  - Ready for: forge test                                      │  │   │
│  │  │  - Immunefi submission format                                 │  │   │
│  │  │                                                                  │  │   │
│  │  └─────────────────────────────────────────────────────────────────┘  │   │
│  │                                                                        │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## What This Replaces

| Old System Component | New System Equivalent | Status |
|---------------------|----------------------|--------|
| ForkerAgent (live forking) | → Integrated into AutonomousEconomicDominator | TO BE ADDED |
| Fetcher (transaction replay) | → Integrated into AutonomousEconomicDominator | TO BE ADDED |
| Hypothesis Generation | → Integrated into AutonomousEconomicDominator | TO BE ADDED |
| Contract Classifier | → Integrated into AutonomousEconomicDominator | TO BE ADDED |
| Old analysis methods | → analyze_autonomous_economic() | TO BE ADDED |

---

## Test Validation Plan

### Before Replacement Valid:
1. Replay LIVE AMMO successful transactions with new system
2. Validate LIVE FORK HIJACK still works  
3. Test Phase 7 Complete (Balancer Vault) analysis
4. Compare detection rates: OLD vs NEW

### After Replacement Valid:
1. Live forking works correctly
2. Transaction replay works correctly
3. Hypothesis generation triggers correctly
4. Economic analysis triggers on financial contracts
5. Multi-contract analysis detects bugs
6. PoC generation produces valid tests

---

*Last Updated: 2026-05-05*
*Version: 3.0*
*Status: Phase 1 - Starting Integration*
*Goal: Permanently Replace Old System with New Enhanced Version*