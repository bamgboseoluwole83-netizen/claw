// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================================
// TEST CONTRACT 1: Cross-Contract Reentrancy (Multi-Contract Financial Bug)
// ============================================================================
// Bug: Reentrancy that spans multiple contracts - attacker calls contract A
// which calls contract B, allowing state changes after external call returns

interface ICallbackReceiver {
    function onReceive() external;
}

contract CrossContractAttacker {
    address public target;
    address public callbackContract;
    uint256 public balance;
    bool public attacked;
    
    constructor(address _target, address _callback) {
        target = _target;
        callbackContract = _callback;
    }
    
    function attack() external payable {
        require(msg.value >= 1 ether, "Need at least 1 ETH");
        attacked = true;
        // Call vulnerable function that has callback
        (bool success, ) = target.call{value: msg.value}(abi.encodeWithSignature("executeWithCallback(address)", callbackContract));
        require(success, "Attack failed");
    }
    
    receive() external payable {
        balance += msg.value;
        // Reentrancy: call callback which can trigger more state changes
        if (callbackContract != address(0)) {
            ICallbackReceiver(callbackContract).onReceive();
        }
    }
}

contract VulnerableCrossContract {
    mapping(address => uint256) public balances;
    address public callbackHandler;
    bool public stateChanged;
    
    function setCallbackHandler(address _handler) external {
        callbackHandler = _handler;
    }
    
    function executeWithCallback(address callback) external payable {
        balances[msg.sender] += msg.value;
        
        // External call - vulnerability point
        if (callback != address(0)) {
            ICallbackReceiver(callback).onReceive();
        }
        
        // State should be changed after callback, but reentrancy allows
        // attacker to manipulate state before this executes
        stateChanged = true;
    }
    
    function deposit() external payable {
        balances[msg.sender] += msg.value;
    }
    
    function withdraw() external {
        uint256 amount = balances[msg.sender];
        balances[msg.sender] = 0;
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success);
    }
}

contract CallbackHandler is ICallbackReceiver {
    address public attacker;
    bool public called;
    
    constructor(address _attacker) {
        attacker = _attacker;
    }
    
    function onReceive() external override {
        called = true;
        // Attacker's receive() gets called, allowing balance manipulation
    }
}