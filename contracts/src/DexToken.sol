// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "solmate/tokens/ERC20.sol";

/// @title DexToken
/// @notice ERC20 token with protocol-level transfer capability for the enshrined DEX
/// @dev Extends solmate's ERC20 with protocolTransfer for approval-free DEX operations
contract DexToken is ERC20 {
    /// @notice The DEX predeploy address that has special transfer privileges
    address public constant DEX_CONTRACT =
        0x4200000000000000000000000000000000000042;

    /// @notice Error thrown when non-DEX address tries to call protocolTransfer
    error OnlyDEX();

    constructor(
        string memory name,
        string memory symbol,
        uint8 decimals_
    ) ERC20(name, symbol, decimals_) {}

    /// @notice Mint tokens to an address (for testing)
    /// @param to The recipient address
    /// @param amount The amount to mint
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    /// @notice Protocol-level transfer that bypasses approvals
    /// @dev Can only be called by the DEX predeploy contract
    /// @param from The address to transfer from
    /// @param to The address to transfer to
    /// @param amount The amount to transfer
    function protocolTransfer(
        address from,
        address to,
        uint256 amount
    ) external {
        if (msg.sender != DEX_CONTRACT) revert OnlyDEX();

        // Directly modify balances (same as internal _transfer)
        balanceOf[from] -= amount;

        // Cannot overflow because the sum of all user balances can't exceed max uint256
        unchecked {
            balanceOf[to] += amount;
        }

        emit Transfer(from, to, amount);
    }
}
