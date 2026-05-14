// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "forge-std/console2.sol";

import "../contracts/HardTest.sol";
import "../contracts/ChallengeLending.sol";
import "../contracts/CreamLending.sol";
import "../contracts/YieldVault.sol";
import "../contracts/MockOracle.sol";
import "../contracts/StaleOracleLender.sol";
import "../contracts/ZeroDayChallenge.sol";
import "../contracts/Attacker.sol";
import "../contracts/VulnerableLender.sol";
import "../contracts/test_cross_reentrancy.sol";
import "../contracts/test_economic_bugs.sol";
import "../contracts/test_flashloan_atomicity.sol";
import "../contracts/test_liquidation_cascade.sol";
import "../contracts/test_oracle_manipulation.sol";
import "../contracts/test_state_diff.sol";

contract DeployTestSuite is Script {
    function run() external {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerKey);

        _deployHardTest();
        _deployChallenge();
        _deployCreamYield();
        _deployMockStale();
        _deployZeroDay();
        _deployCrossContract();
        _deployEconomicBugs();
        _deployFlashLoan();
        _deployCascade();
        _deployOracleManip();
        _deployStateDiff();

        vm.stopBroadcast();
    }

    function _deployHardTest() internal {
        HardOracleImpl impl = new HardOracleImpl();
        HardOracleProxy proxy = new HardOracleProxy(address(impl));
        HardLender lender = new HardLender(address(proxy));
        console2.log("HardOracleImpl:", address(impl));
        console2.log("HardOracleProxy:", address(proxy));
        console2.log("HardLender:", address(lender));
    }

    function _deployChallenge() internal {
        OracleImpl impl = new OracleImpl();
        OracleProxy proxy = new OracleProxy(address(impl));
        ChallengeLending lender = new ChallengeLending(address(proxy));
        console2.log("ChallengeImpl:", address(impl));
        console2.log("ChallengeProxy:", address(proxy));
        console2.log("ChallengeLender:", address(lender));
    }

    function _deployCreamYield() internal {
        YieldVault vault = new YieldVault();
        CreamLending cream = new CreamLending(address(vault));
        console2.log("YieldVault:", address(vault));
        console2.log("CreamLending:", address(cream));
    }

    function _deployMockStale() internal {
        MockOracle oracle = new MockOracle();
        StaleOracleLender lender = new StaleOracleLender(address(oracle));
        console2.log("MockOracle:", address(oracle));
        console2.log("StaleOracleLender:", address(lender));
    }

    function _deployZeroDay() internal {
        VulnerableVault vault = new VulnerableVault();
        VulnerablePool pool = new VulnerablePool();
        console2.log("ZeroDayVault:", address(vault));
        console2.log("ZeroDayPool:", address(pool));

        Attacker attacker = new Attacker(address(vault));
        console2.log("ZeroDayAttacker:", address(attacker));

        VulnerableLender lender = new VulnerableLender();
        console2.log("VulnerableLender:", address(lender));
    }

    function _deployCrossContract() internal {
        VulnerableCrossContract target = new VulnerableCrossContract();
        CrossContractAttacker attacker = new CrossContractAttacker(address(target), address(0));
        CallbackHandler handler = new CallbackHandler(address(attacker));
        console2.log("CrossContractTarget:", address(target));
        console2.log("CrossContractAttacker:", address(attacker));
        console2.log("CrossContractCallback:", address(handler));
    }

    function _deployEconomicBugs() internal {
        VulnerablePriceOracleSimple oracle = new VulnerablePriceOracleSimple();
        SafePriceOracle safeOracle = new SafePriceOracle();
        VulnerableLiquidationSimple liq = new VulnerableLiquidationSimple();
        VulnerableUndercollateralized under = new VulnerableUndercollateralized();
        StorageSlotA slotA = new StorageSlotA();
        StorageSlotB slotB = new StorageSlotB();
        VulnerableFlashLoanSimple flash = new VulnerableFlashLoanSimple();
        SafeFlashLoan safeFlash = new SafeFlashLoan();
        console2.log("EcoOracle:", address(oracle));
        console2.log("EcoSafeOracle:", address(safeOracle));
        console2.log("EcoLiquidation:", address(liq));
        console2.log("EcoUnderCollat:", address(under));
        console2.log("EcoSlotA:", address(slotA));
        console2.log("EcoSlotB:", address(slotB));
        console2.log("EcoFlashLoan:", address(flash));
        console2.log("EcoSafeFlash:", address(safeFlash));
    }

    function _deployFlashLoan() internal {
        SimpleFlashLoan simple = new SimpleFlashLoan();
        VulnerableExchange exchange = new VulnerableExchange();
        FlashLoanAtomicityAttacker attacker = new FlashLoanAtomicityAttacker(address(simple), address(exchange));
        RealisticFlashLoanAttacker realistic = new RealisticFlashLoanAttacker(address(0), address(0));
        console2.log("FlashLoanSimple:", address(simple));
        console2.log("FlashLoanExchange:", address(exchange));
        console2.log("FlashLoanAtomicityAttacker:", address(attacker));
        console2.log("FlashLoanRealistic:", address(realistic));
    }

    function _deployCascade() internal {
        CascadeToken token = new CascadeToken();
        VulnerableLendingA a = new VulnerableLendingA(address(0));
        VulnerableLendingB b = new VulnerableLendingB(address(0), address(a));
        CascadeAttacker attacker = new CascadeAttacker(address(a), address(b), address(0));
        console2.log("CascadeToken:", address(token));
        console2.log("CascadeLendingA:", address(a));
        console2.log("CascadeLendingB:", address(b));
        console2.log("CascadeAttacker:", address(attacker));
    }

    function _deployOracleManip() internal {
        VulnerablePriceOracle oracle = new VulnerablePriceOracle();
        VulnerableLendingProtocol lending = new VulnerableLendingProtocol(address(oracle));
        OracleManipulationAttacker attacker = new OracleManipulationAttacker(address(oracle), address(lending));
        console2.log("OracleManipOracle:", address(oracle));
        console2.log("OracleManipLending:", address(lending));
        console2.log("OracleManipAttacker:", address(attacker));
    }

    function _deployStateDiff() internal {
        ProtocolB b = new ProtocolB(address(0));
        ProtocolA a = new ProtocolA(address(b));
        StateDiffAttacker attacker = new StateDiffAttacker(address(a), address(b));
        console2.log("StateDiffA:", address(a));
        console2.log("StateDiffB:", address(b));
        console2.log("StateDiffAttacker:", address(attacker));
    }
}
