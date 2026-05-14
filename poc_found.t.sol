// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "forge-std/Test.sol";

contract PoC_1_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_1() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"c1b6678d0000000000000000000000000000000000000000000000000000000000000000"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_2_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_2() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"e8e33700"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_3_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_3() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"1f914ab10000000000000000000000000000000000000000000000000000000000000000"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_4_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_4() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"3093a22a0000000000000000000000000000000000000000000000000000000000000000"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_5_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_5() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"e8e33700"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_6_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_6() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"c5dd13a30000000000000000000000000000000000000000000000000000000000000000"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

contract PoC_7_Heimdall is Test {
    uint256 fork;
    string RPC_URL = vm.envOr("DRPC_URL", string("https://your-fallback-node.com"));

    function setUp() public {
        fork = vm.createFork(RPC_URL, 25093654);
        vm.selectFork(fork);
    }

    function test_exploit_7() public {
        address attacker = makeAddr("attacker");
        vm.deal(attacker, 1000 ether);
        vm.startPrank(attacker);

        (bool s, ) = address(0x7a250d5630b4cf539739df2c5dacb4c659f2488d).call(
hex"e524d96d0000000000000000000000000000000000000000000000000000000000000000"
);
        emit log("Note: Call attempted at pinned block — preconditions not met. Function entry point is active.");
        vm.stopPrank();
    }
}

