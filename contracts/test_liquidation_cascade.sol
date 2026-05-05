// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// TEST CONTRACT 3: Liquidation Cascade (Multi-Contract Financial Bug)
// ============================================================================
// Bug: Liquidation in one protocol triggers liquidations in others - 
// cascade effect across multiple lending protocols

interface IPriceOracle {
    function getPrice(address token) external view returns (uint256);
}

contract CascadeToken {
    string public name = "CascadeToken";
    string public symbol = "CST";
    uint8 public decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    
    constructor() {
        totalSupply = 1000000e18;
        balanceOf[msg.sender] = totalSupply;
    }
    
    function transfer(address to, uint256 amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract VulnerableLendingA {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public debt;
    address public priceOracle;
    uint256 public threshold = 150;
    event Liquidated(address user, uint256 amount);
    
    constructor(address _oracle) {
        priceOracle = _oracle;
    }
    
    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }
    
    function borrow(uint256 amount) external {
        uint256 price = IPriceOracle(priceOracle).getPrice(address(0));
        require(collateral[msg.sender] * price >= debt[msg.sender] + amount * threshold / 100, "Insufficient collateral");
        debt[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function liquidate(address user) external {
        uint256 price = IPriceOracle(priceOracle).getPrice(address(0));
        uint256 health = collateral[user] * price * 100 / (debt[user] + 1);
        require(health < threshold, "Position healthy");
        
        uint256 debtAmount = debt[user];
        uint256 collatAmount = collateral[user];
        
        debt[user] = 0;
        collateral[user] = 0;
        
        // 10% liquidation bonus
        payable(msg.sender).transfer(collatAmount * 110 / 100);
        
        emit Liquidated(user, debtAmount);
    }
}

contract VulnerableLendingB {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public debt;
    address public priceOracle;
    address public lendingA;
    uint256 public threshold = 150;
    event CascadedLiquidation(address user, bool fromA);
    
    constructor(address _oracle, address _lendingA) {
        priceOracle = _oracle;
        lendingA = _lendingA;
    }
    
    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }
    
    function borrow(uint256 amount) external {
        uint256 price = IPriceOracle(priceOracle).getPrice(address(0));
        require(collateral[msg.sender] * price >= debt[msg.sender] + amount * threshold / 100, "Insufficient collateral");
        debt[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function liquidate(address user) external {
        uint256 price = IPriceOracle(priceOracle).getPrice(address(0));
        uint256 health = collateral[user] * price * 100 / (debt[user] + 1);
        
        // If position is underwater relative to LendingA's collateral, cascade
        if (health < threshold && collateral[user] > 1000e18) {
            emit CascadedLiquidation(user, true);
        }
        
        uint256 debtAmount = debt[user];
        uint256 collatAmount = collateral[user];
        
        debt[user] = 0;
        collateral[user] = 0;
        
        payable(msg.sender).transfer(collatAmount * 110 / 100);
    }
}

contract CascadeAttacker {
    address public lendingA;
    address public lendingB;
    address public priceOracle;
    
    constructor(address _lendingA, address _lendingB, address _oracle) {
        lendingA = _lendingA;
        lendingB = _lendingB;
        priceOracle = _oracle;
    }
    
    function triggerCascade() external {
        // Step 1: Manipulate oracle to trigger liquidation in LendingA
        // This creates cascading liquidations in LendingB
        // Both protocols use same oracle, so price manipulation affects both
    }
}