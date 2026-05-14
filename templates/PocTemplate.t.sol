// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

contract {{TEST_NAME}} is Test {
    address constant TARGET = {{TARGET_ADDRESS}};
    address constant ATTACKER = {{ATTACKER_ADDRESS}};
    uint256 constant BLOCK_NUMBER = {{BLOCK_NUMBER}};

    function setUp() public {
        // Fork the chain to get real state
        vm.createSelectFork(
            vm.envString("DRPC_URL"),
            BLOCK_NUMBER == 0 ? block.number : BLOCK_NUMBER
        );
    }

    /// Exploit: {{STRATEGY}}
    /// Estimated profit: {{PROFIT_ESTIMATE_ETH}} ETH
    function testExploit() public {
        vm.startPrank(ATTACKER);

        {{PRE_BALANCE_CODE}}

        // ── Exploit Steps ──
        {{STEP_CODE}}

        {{POST_BALANCE_CODE}}

        vm.stopPrank();

        // Verify profit
        {{PROFIT_ASSERT}}
    }
}
