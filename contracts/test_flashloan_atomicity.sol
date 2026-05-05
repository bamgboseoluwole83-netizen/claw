// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// TEST CONTRACT 4: Flash Loan Atomicity Violation (Multi-Contract Financial Bug)
// ============================================================================
// Bug: Flash loan exploits that break atomicity assumptions - attacker
// executes multi-step attack that fails partially but keeps profits

interface IFlashLoanReceiver {
    function executeFlashLoan(uint256 amount) external;
}

contract SimpleFlashLoan {
    mapping(address => uint256) public reserves;
    
    function deposit() external payable {
        reserves[msg.sender] += msg.value;
    }
    
    function flashLoan(address receiver, uint256 amount) external {
        require(reserves[address(this)] >= amount, "Insufficient reserves");
        
        // Send flash loan
        payable(receiver).transfer(amount);
        
        // Atomicity violation: If repayment doesn't happen, 
        // loan is still considered successful but funds are gone
    }
    
    receive() external payable {
        reserves[msg.sender] += msg.value;
    }
}

contract VulnerableExchange {
    uint256 public price;
    mapping(address => uint256) public balances;
    
    function setPrice(uint256 _price) external {
        price = _price;
    }
    
    function swap(address tokenIn, address tokenOut, uint256 amountIn) external returns (uint256) {
        require(price > 0, "Price not set");
        uint256 amountOut = amountIn * price / 1e18;
        balances[tokenOut] -= amountOut;
        balances[tokenIn] += amountIn;
        return amountOut;
    }
    
    function getBalance(address token) external view returns (uint256) {
        return balances[token];
    }
}

contract FlashLoanAtomicityAttacker {
    address public flashLoan;
    address public exchange;
    uint256 public profit;
    bool public attacked;
    
    constructor(address _flashLoan, address _exchange) {
        flashLoan = _flashLoan;
        exchange = _exchange;
    }
    
    function attack() external {
        // VULNERABILITY: Multi-step attack with partial failure
        // If any step fails, previous steps aren't reverted properly
        
        // Step 1: Get flash loan
        (bool success, ) = flashLoan.delegatecall(abi.encodeWithSignature("flashLoan(address,uint256)", address(this), 1000 ether));
        
        // VULNERABILITY: Even if flash loan fails, attacker might have
        // already extracted value from previous operations
        require(success, "Flash loan failed");
        
        attacked = true;
    }
    
    receive() external payable {
        // Step 2: During flash loan, manipulate exchange
        VulnerableExchange(exchange).setPrice(1); // Manipulate price
        
        // Step 3: Execute attack logic
        // If this fails, flash loan should revert - but atomicity is broken
        // allowing partial extraction
    }
}

// More realistic flash loan atomicity violation
contract RealisticFlashLoanAttacker {
    address public targetProtocol;
    address public uniswapRouter;
    uint256 public profit;
    
    constructor(address _target, address _uniswap) {
        targetProtocol = _target;
        uniswapRouter = _uniswap;
    }
    
    function attack() external payable {
        // Step 1: Flash loan 10M USDC
        // Step 2: Swap USDC -> ETH on Uniswap (manipulate price)
        // Step 3: Swap back ETH -> USDC (exploit manipulated price)
        // Step 4: Repay flash loan
        
        // ATOMICITY VIOLATION: If step 4 fails (repayment), 
        // attacker keeps profit from steps 2-3 because:
        // - MEV extractor may have already extracted value
        // - Multiple tx execution means partial success = profit
        // - Flash loan can be repaid from profit, leaving net positive
        
        // Attack would work by:
        // 1. Execute arbitrage in same block as flash loan
        // 2. Even if liquidation or revert happens, MEV captures profit
        // 3. Net result: attacker keeps profit, protocol loses
        
        profit = msg.value * 2; // Simplified - real attack would extract more
    }
    
    receive() external payable {}
}