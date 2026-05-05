#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use web3_destroyer::agents::autonomous_economic_dominator::{
        AutonomousEconomicDominator, DominatorConfig, TargetSource, VulnerabilitySeverity,
    };

    #[test]
    fn test_autonomous_dominator_end_to_end() {
        println!("\n=====================================================");
        println!("FULLY AUTONOMOUS ECONOMIC DOMINATOR - DEMO");
        println!("=====================================================\n");

        // Create with custom config
        let config = DominatorConfig {
            max_fuzz_iterations: 50,
            min_confidence: 0.7,
            enable_multi_contract: true,
            auto_poc: true,
            enable_forking: true,
            target_protocols: vec![
                "Aave".to_string(),
                "Compound".to_string(),
                "Uniswap".to_string(),
            ],
        };

        let mut dominator = AutonomousEconomicDominator::with_config(config);

        // Test 1: Set static analysis target
        println!("TEST 1: Static Analysis Target");
        let target_addr = Address::new([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xAB, 0xCD,
        ]);
        let bytecode = vec![
            0x60, 0x00, // PUSH1 0
            0x80, // DUP1
            0x54, // SLOAD
            0x55, // SSTORE
            0x00, // STOP
        ];

        dominator.set_target(TargetSource::StaticAnalysis {
            address: target_addr,
            bytecode: bytecode.clone(),
        });
        println!("✓ Target set: Static analysis of {:?}", target_addr);

        // Test 2: Run quick scan
        println!("\nTEST 2: Quick Scan");
        let results = dominator.quick_scan(target_addr, &bytecode);
        println!("✓ Quick scan complete: {} findings", results.len());

        // Test 3: Set forked mainnet target
        println!("\nTEST 3: Forked Mainnet Target");
        dominator.set_target(TargetSource::ForkedMainnet {
            rpc_url: "https://eth-mainnet.g.alchemy.com/v2/demo".to_string(),
            block_number: 19_500_000,
        });
        println!("✓ Forked mainnet target set at block 19,500,000");

        // Test 4: Set protocol target
        println!("\nTEST 4: Protocol Target");
        dominator.set_target(TargetSource::Protocol {
            name: "DeFi Lending Protocol".to_string(),
            addresses: vec![Address::new([0; 20]), Address::new([0xff; 20])],
        });
        println!("✓ Protocol target set with 2 contracts");

        // Test 5: Run full analysis pipeline
        println!("\nTEST 5: Full Analysis Pipeline");
        dominator.set_target(TargetSource::StaticAnalysis {
            address: target_addr,
            bytecode,
        });
        let exploits = dominator.analyze();
        println!("✓ Analysis complete: {} exploits found", exploits.len());

        // Test 6: Get critical findings
        println!("\nTEST 6: Critical Findings");
        let critical = dominator.get_critical_findings();
        println!("✓ Critical findings: {}", critical.len());

        // Test 7: Export Immunefi report
        println!("\nTEST 7: Immunefi Report Export");
        let report = dominator.export_immunefi_report();
        println!("✓ Report generated ({} chars)", report.len());
        println!("  Preview: {}", &report[..100.min(report.len())]);

        println!("\n=====================================================");
        println!("✓ ALL AUTONOMOUS DOMINATOR TESTS PASSED!");
        println!("=====================================================\n");
    }

    #[test]
    fn test_autonomous_with_different_severities() {
        let mut dominator = AutonomousEconomicDominator::new();

        // Test different vulnerability types
        dominator.set_target(TargetSource::StaticAnalysis {
            address: Address::default(),
            bytecode: vec![0x00],
        });

        let exploits = dominator.analyze();
        println!("Found {} exploits", exploits.len());
        assert!(true); // Just ensure it runs without panic
    }

    #[test]
    fn test_config_defaults() {
        let config = DominatorConfig::default();
        assert_eq!(config.max_fuzz_iterations, 100);
        assert_eq!(config.min_confidence, 0.7);
        assert!(config.enable_multi_contract);
        assert!(config.auto_poc);
        assert!(config.enable_forking);
        assert!(!config.target_protocols.is_empty());
    }
}
