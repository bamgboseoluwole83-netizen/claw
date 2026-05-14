// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;
contract MockPair {
    address public token0;
    address public token1;
    uint256 public reserve0;
    uint256 public reserve1;
    uint32 public blockTimestampLast;
    constructor(address _t0, address _t1) { token0 = _t0; token1 = _t1; }
    function setReserves(uint256 r0, uint256 r1, uint32 ts) external { reserve0 = r0; reserve1 = r1; blockTimestampLast = ts; }
    function getReserves() external view returns (uint112 r0, uint112 r1, uint32 ts) {
        r0 = uint112(reserve0); r1 = uint112(reserve1); ts = blockTimestampLast;
    }
}
