pub mod forker;
pub mod execution_agent;
pub mod symbolic_stack;
pub mod assassin_engine;
pub mod precision_solver;
pub mod exploit_verifier;
pub mod fetcher;
pub mod simulator;
pub mod reentrancy_detector;
pub mod ror_optimizer;
pub mod reward_dilution;
pub mod oracle_laggard;
pub mod oracle_discovery;
pub mod poc_generator;
pub mod repo_scanner;
pub mod solver;
pub mod notifier;
pub mod proxy_resolver;
pub mod evmole_analyzer;
pub mod ityfuzz_integration;
pub mod orchestrator;
pub mod storage_tracer;
pub mod source_fetcher;
pub mod constructor_tool;
pub mod revenue_calc;
pub mod severity;
pub mod contract_classifier;
pub mod invariant_generator;
pub mod formal_invariant_engine;
pub mod symbolic_executor;
pub mod bitwuzla_solver;
pub mod symbolic_interpreter;
pub mod economic_engine;
pub mod concolic_engine;  // Hybrid execution engine (Phase 1 + navigator)
pub mod discovery_engine; // Phase 5: Custom revm fuzzer
pub mod libafl_harness;    // Phase 5: Proper LibAFL differential feedback
pub mod trace_flattener;    // Phase 5: Trace-to-constraint bridge
pub mod multi_contract_analysis; // Phase 3: Cross-protocol analysis
pub mod context_first_engine; // NEW: Layer 1-3 Context-first execution
pub mod autonomous_economic_dominator; // NEW: Fully Autonomous Economic Dominator
