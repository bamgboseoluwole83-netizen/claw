// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// TEST CONTRACT 5: State Diff Exploit (Multi-Contract Financial Bug)
// ============================================================================
// Bug: Inconsistencies in financial state across protocols - balance sheet
// doesn't add up, allowing extraction of value

contract ProtocolA {
    uint256 public totalDeposits;
    uint256 public totalBorrows;
    mapping(address => uint256) public deposits;
    mapping(address => uint256) public borrows;
    address public sisterProtocol;
    
    constructor(address _sister) {
        sisterProtocol = _sister;
    }
    
    function deposit() external payable {
        deposits[msg.sender] += msg.value;
        totalDeposits += msg.value;
        
        // Cross-protocol state update
        // STATE DIFF VULNERABILITY: If this fails but deposit succeeds,
        // totalDeposits doesn't match actual holdings
    }
    
    function borrow(uint256 amount) external {
        require(deposits[msg.sender] >= amount * 150 / 100, "Insufficient collateral");
        borrows[msg.sender] += amount;
        totalBorrows += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function getStateConsistency() external view returns (bool) {
        // STATE DIFF: Check if total deposits - borrows = actual balance
        // This can be manipulated if sister protocol state is inconsistent
        return (totalDeposits - totalBorrows) <= address(this).balance;
    }
    
    receive() external payable {}
}

contract ProtocolB {
    uint256 public totalDeposits;
    uint256 public totalBorrows;
    mapping(address => uint256) public deposits;
    mapping(address => uint256) public borrows;
    address public sisterProtocol;
    
    constructor(address _sister) {
        sisterProtocol = _sister;
    }
    
    function deposit() external payable {
        deposits[msg.sender] += msg.value;
        totalDeposits += msg.value;
    }
    
    function borrow(uint256 amount) external {
        require(deposits[msg.sender] >= amount * 150 / 100, "Insufficient collateral");
        borrows[msg.sender] += amount;
        totalBorrows += amount;
        payable(msg.sender).transfer(amount);
    }
    
    function getStateConsistency() external view returns (bool) {
        return (totalDeposits - totalBorrows) <= address(this).balance;
    }
    
    receive() external payable {}
}

// State diff exploit: Extract value by exploiting inconsistent state between protocols
contract StateDiffAttacker {
    address public protocolA;
    address public protocolB;
    
    constructor(address _A, address _B) {
        protocolA = _A;
        protocolB = _B;
    }
    
    function exploit() external {
        // STATE DIFFERENCE EXPLOIT:
        // Both protocols should maintain: deposits - borrows = real balance
        // But if one protocol's state is manipulated (e.g., through reentrancy),
        // the difference can be extracted
        
        // Exploit:
        // 1. Deposit into Protocol A
        // 2. If Protocol A state is manipulated to show higher balance than reality
        // 3. Borrow from Protocol B using inflated collateral
        // 4. Protocol B's view of collateral is inconsistent with Protocol A's actual state
        // 5. Extract value before inconsistency is discovered
        
        // This is the basis of many real exploits like Euler Finance
    }
    
    receive() external payable {}
}