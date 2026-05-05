// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// ECONOMIC BUG TEST CONTRACTS - Context-First Engine Detection
// ============================================================================

// ============================================================================
// 1. Price Manipulation (Economic Bug - Context-First Engine)
// ============================================================================
contract VulnerablePriceOracleSimple {
    // VULNERABILITY: No access control - anyone can set price!
    uint256 public price;
    address public owner;
    
    constructor() {
        owner = msg.sender;
        price = 1000e8; // $1000 initial
    }
    
    // BUG: No access control check - should have onlyOwner modifier
    function setPrice(uint256 _price) external {
        price = _price;
    }
    
    function getPrice() external view returns (uint256) {
        return price;
    }
}

contract SafePriceOracle {
    address public owner;
    uint256 public price;
    uint256 public lastUpdate;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
        price = 1000e8;
        lastUpdate = block.timestamp;
    }
    
    function setPrice(uint256 _price) external onlyOwner {
        price = _price;
        lastUpdate = block.timestamp;
    }
    
    function getPrice() external view returns (uint256) {
        return price;
    }
}

// ============================================================================
// 2. Liquidation Bypass (Economic Bug - Context-First Engine)  
// ============================================================================
contract VulnerableLiquidationSimple {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public borrowed;
    uint256 public constant THRESHOLD = 150; // 150%
    
    // BUG: Health factor check allows liquidations when health > 1.0
    function getHealthFactor(address user) public view returns (uint256) {
        if (borrowed[user] == 0) return type(uint256).max;
        return collateral[user] * 100 / borrowed[user];
    }
    
    function liquidate(address user) external {
        uint256 health = getHealthFactor(user);
        
        // BUG: Should be health < 100, but allows health = 150 (above 1.0)
        require(health < THRESHOLD, "Health too high");
        
        uint256 debt = borrowed[user];
        uint256 collat = collateral[user];
        
        borrowed[user] = 0;
        collateral[user] = 0;
        
        // Liquidator gets 10% bonus
        payable(msg.sender).transfer(collat * 110 / 100);
    }
    
    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }
    
    function borrow(uint256 amount) external {
        require(getHealthFactor(msg.sender) >= THRESHOLD, "Insufficient collateral");
        borrowed[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
    
    receive() external payable {}
}

// ============================================================================
// 3. Borrow Undercollateralized (Economic Bug - Context-First Engine)
// ============================================================================
contract VulnerableUndercollateralized {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public borrowed;
    uint256 public price = 1000e8;
    
    // BUG: No proper collateral check - allows borrowing more than collateral
    function borrow(uint256 amount) external {
        // Simple check that's easily bypassable
        if (collateral[msg.sender] > 0) {
            // BUG: Should multiply by price, but uses direct comparison
            // allowing undercollateralized loans
            require(collateral[msg.sender] >= amount, "Not enough collateral");
        } else {
            // BUG: Can bypass with zero collateral if this branch is reached
            // because collateral check is skipped when collateral is 0
            require(borrowed[msg.sender] + amount <= 1, "Can only borrow 1 wei with 0 collateral");
        }
        
        borrowed[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }
    
    receive() external payable {}
}

// ============================================================================
// 4. Storage Slot Overlap (Phase 1 - Navigator Detection)
// ============================================================================
contract StorageSlotA {
    uint256 public value1;  // Slot 0
    uint256 public value2;  // Slot 1
    mapping(address => uint256) public balances; // Slot keccak256(p)+0
    
    function setValues(uint256 _v1, uint256 _v2) external {
        value1 = _v1;
        value2 = _v2;
    }
}

contract StorageSlotB {
    // VULNERABILITY: This contract uses same slot 0 for 'owner'
    // as StorageSlotA uses for 'value1' - if deployed at same address,
    // they can overwrite each other's data!
    address public owner; // Slot 0 - CONFLICTS WITH StorageSlotA.value1!
    uint256 public count;  // Slot 1 - CONFLICTS WITH StorageSlotA.value2!
    
    function setOwner(address _owner) external {
        owner = _owner;
    }
    
    function setCount(uint256 _count) external {
        count = _count;
    }
}

// ============================================================================
// 5. Flash Loan Simple (Economic Bug - Context-First Engine)
// ============================================================================
contract VulnerableFlashLoanSimple {
    mapping(address => uint256) public balances;
    uint256 public fee = 1e18; // 1% fee
    
    function flashLoan(uint256 amount) external {
        require(balances[address(this)] >= amount, "Insufficient liquidity");
        
        // BUG: Send funds without any callback or repayment check
        payable(msg.sender).transfer(amount);
        
        // Should require repayment in same transaction!
        // BUG: Missing repayment check allows stealing funds
    }
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
    
    receive() external payable {}
}

contract SafeFlashLoan {
    mapping(address => uint256) public balances;
    uint256 public fee = 1e18;
    
    function flashLoan(address receiver, uint256 amount) external {
        require(balances[address(this)] >= amount, "Insufficient liquidity");
        
        uint256 balanceBefore = address(this).balance;
        
        // Send to receiver
        payable(receiver).transfer(amount);
        
        // Must be repaid in same tx (Solidity ensures this)
        require(address(this).balance >= balanceBefore + fee, "Not repaid");
    }
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
    
    receive() external payable {}
}