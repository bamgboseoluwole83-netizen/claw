// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract HardOracleImpl {
    uint256 public price;
    uint256 public nonce;

    function setPrice(uint256 newPrice) external {
        price = newPrice;
    }

    function prepare() external {
        nonce++;
    }

    function setPriceGated(uint256 newPrice) external {
        require(nonce > 0, "nonce");
        nonce--;
        price = newPrice;
    }

    function getPrice() external view returns (uint256) {
        return price;
    }

    function pricePerShare() external view returns (uint256) {
        return price;
    }

    function latestAnswer() external view returns (uint256) {
        return price;
    }

    function reset() external {
        price = 0;
        nonce = 0;
    }
}

contract HardOracleProxy {
    address public implementation;

    constructor(address _impl) {
        implementation = _impl;
    }

    fallback(bytes calldata data) external payable returns (bytes memory) {
        (bool ok, bytes memory result) = implementation.delegatecall(data);
        require(ok);
        return result;
    }
}

contract HardLender {
    address private oracle;

    mapping(address => uint256) public deposits;

    constructor(address _oracle) {
        oracle = _oracle;
    }

    receive() external payable {}

    function deposit() external payable {
        deposits[msg.sender] += msg.value;
    }

    function borrow(uint256 amount) external {
        (bool ok, bytes memory result) = oracle.staticcall(
            abi.encodeWithSignature("getPrice()")
        );
        require(ok);
        uint256 price_ = abi.decode(result, (uint256));
        require(price_ > 0, "price zero");

        uint256 maxBorrow = (deposits[msg.sender] * price_) / 1e18;
        require(amount <= maxBorrow, "exceeds collateral");
        require(amount <= address(this).balance, "insufficient liquidity");

        deposits[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }

    function withdrawAll() external {
        payable(msg.sender).transfer(address(this).balance);
    }
}