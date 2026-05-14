"""
web3-destroyer Custom Slither Detectors
Targets high-value contest patterns: oracle manipulation,
rounding attacks, flash loan + reentrancy combos.

Usage: slither . --detect oracle-twap,rounding-direction,flash-reentrancy --json results.json
"""

from slither.detectors.abstract_detector import AbstractDetector, DetectorClassification
from slither.slithir.operations import (
    HighLevelCall,
    LowLevelCall,
    Send,
    Transfer,
    SolidityCall,
)
from slither.core.cfg.node import NodeType

# ═══════════════════════════════════════════
# Detector 1: Oracle TWAP Manipulation
# ═══════════════════════════════════════════


class OracleTWAPManipulation(AbstractDetector):
    ARGUMENT = "oracle-twap"
    HELP = "Detects contracts using spot price from a DEX as an oracle without TWAP"
    IMPACT = DetectorClassification.HIGH
    CONFIDENCE = DetectorClassification.MEDIUM

    WIKI = "https://github.com/your-org/web3-destroyer"
    WIKI_TITLE = "Oracle TWAP Manipulation"
    WIKI_DESCRIPTION = "Contract reads spot price from a single AMM pool. An attacker can flash loan to manipulate the pool and extract value."
    WIKI_EXPLOIT_SCENARIO = "Alice deposits 100 ETH using manipulated oracle price. Bob flash loans 10M DAI, swaps on pool, drives price up 20x, liquidates everyone."
    WIKI_RECOMMENDATION = "Use a TWAP oracle (Uniswap V2 accumulate() or Chainlink) instead of spot price."

    def _detect(self):
        results = []
        for contract in self.slither.contracts_derived:
            for function in contract.functions:
                for node in function.nodes:
                    for ir in node.irs:
                        if isinstance(ir, HighLevelCall):
                            if ir.function_name in (
                                "getReserves",
                                "getAmountsOut",
                                "getAmountIn",
                            ):
                                if not self._has_twap_check(function):
                                    info = [
                                        f"Oracle spot price read in {function.name}()\n",
                                        f"\t- Uses {ir.function_name} which returns **spot price** (manipulable)\n",
                                        f"\t- No TWAP accumulate() or consult() call in same function\n",
                                    ]
                                    res = self.generate_result(info)
                                    results.append(res)
        return results

    def _has_twap_check(self, function):
        for node in function.nodes:
            for ir in node.irs:
                if isinstance(ir, HighLevelCall):
                    if ir.function_name in (
                        "accumulate",
                        "consult",
                        "twap",
                        "price0CumulativeLast",
                        "price1CumulativeLast",
                    ):
                        return True
        return False


# ═══════════════════════════════════════════
# Detector 2: Rounding Direction Favors Attacker
# ═══════════════════════════════════════════


class RoundingDirectionAttacker(AbstractDetector):
    ARGUMENT = "rounding-direction"
    HELP = "Detects mulDiv patterns where rounding benefits the caller in economic operations"
    IMPACT = DetectorClassification.MEDIUM
    CONFIDENCE = DetectorClassification.HIGH

    WIKI = "https://github.com/your-org/web3-destroyer"
    WIKI_TITLE = "Rounding Direction Mismatch"
    WIKI_DESCRIPTION = "Division rounds in favor of the user, allowing small value extraction over many transactions"
    WIKI_EXPLOIT_SCENARIO = "Vault divides shares rounding UP for deposits and DOWN for withdrawals. Attacker deposits/withdraws repeatedly to drain."
    WIKI_RECOMMENDATION = "Ensure rounding always favors the protocol, not the user."

    def _detect(self):
        results = []
        for contract in self.slither.contracts_derived:
            for function in contract.functions:
                func_source = str(function)
                # Look for division where result is attributed to msg.sender
                if "/" in func_source and (
                    "msg.sender" in func_source
                    or "user" in func_source
                    or "caller" in func_source
                ):
                    # Check if this is a deposit/mint/withdraw function
                    fname = function.name.lower()
                    if any(
                        kw in fname
                        for kw in (
                            "deposit",
                            "withdraw",
                            "mint",
                            "redeem",
                            "stake",
                            "unstake",
                            "borrow",
                            "repay",
                        )
                    ):
                        info = [
                            f"Potential rounding favor in {function.name}()\n",
                            f"\t- Economic function with division affecting caller\n",
                            f"\t- Review rounding direction\n",
                        ]
                        res = self.generate_result(info)
                        results.append(res)
        return results


# ═══════════════════════════════════════════
# Detector 3: Flash Loan + Reentrancy Combo
# ═══════════════════════════════════════════


class FlashLoanReentrancyCombo(AbstractDetector):
    ARGUMENT = "flash-reentrancy"
    HELP = "Detects flash loan callbacks that modify state before checks — critical cross-function reentrancy"
    IMPACT = DetectorClassification.CRITICAL
    CONFIDENCE = DetectorClassification.MEDIUM

    WIKI = "https://github.com/your-org/web3-destroyer"
    WIKI_TITLE = "Flash Loan + Reentrancy Combo"
    WIKI_DESCRIPTION = "Contract calls flashLoan() which triggers a callback on the caller. The callback can reenter before state is updated."
    WIKI_EXPLOIT_SCENARIO = "Flash loan callback calls withdraw() before borrow balance is updated, draining the pool."
    WIKI_RECOMMENDATION = "Apply checks-effects-interactions pattern. Update balances before making external calls."

    def _detect(self):
        results = []
        for contract in self.slither.contracts_derived:
            for function in contract.functions:
                has_flash_loan = False
                has_state_change = False
                for node in function.nodes:
                    for ir in node.irs:
                        if isinstance(ir, HighLevelCall):
                            if "flash" in ir.function_name.lower():
                                has_flash_loan = True
                        if isinstance(ir, SolidityCall):
                            if ir.function_name in (
                                "balanceOf(address)",
                                "transfer(address,uint256)",
                            ):
                                has_state_change = True

                has_callback = any(
                    "call" in node.note.lower() or "onFlashLoan" in function.full_name
                    for node in function.nodes
                )

                if has_flash_loan and (has_callback or has_state_change):
                    info = [
                        f"Flash loan + state change in {function.name}()\n",
                        f"\t- Flash loan detected with state modification\n",
                        f"\t- Potential callback → reenter → drain\n",
                    ]
                    res = self.generate_result(info)
                    results.append(res)
        return results
