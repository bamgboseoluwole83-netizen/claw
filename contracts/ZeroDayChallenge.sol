// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract VulnerableVault {
    mapping(address => uint256) public balances;
    address public owner;
    
    event Deposit(address indexed user, uint256 amount);
    event Withdraw(address indexed user, uint256 amount);
    
    constructor() {
        owner = msg.sender;
    }
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
        emit Deposit(msg.sender, msg.value);
    }
    
    // BUG: Reentrancy - external call BEFORE state update
    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount, "insufficient balance");
        
        // External call FIRST - can reenter before state update
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "transfer failed");
        
        // State updated AFTER external call - vulnerable!
        balances[msg.sender] -= amount;
        
        emit Withdraw(msg.sender, amount);
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
    
    receive() external payable {}
}

contract VulnerablePool {
    uint256 public totalShares;
    mapping(address => uint256) public shares;
    
    // BUG: Integer division causes precision loss
    function deposit(uint256 amount) external payable {
        require(amount > 0, "cannot deposit 0");
        
        uint256 currentDeposits = address(this).balance - msg.value;
        
        if (totalShares == 0) {
            shares[msg.sender] = amount;
            totalShares = amount;
        } else {
            // Division BEFORE multiplication - can cause overflow/underflow
            uint256 newShares = (amount * totalShares) / currentDeposits;
            // Bug: amount too small gets rounded to 0, deposit silently fails
            require(newShares > 0, "amount too small");
            shares[msg.sender] += newShares;
            totalShares += newShares;
        }
    }
    
    function getMyShares() external view returns (uint256) {
        return shares[msg.sender];
    }
    
    receive() external payable {}
}