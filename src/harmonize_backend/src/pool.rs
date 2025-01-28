use std::collections::HashMap;
use candid::Principal;
use ethers_core::types::H160;
use num::integer::Roots;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AssetId {
    Eth(u32),
    Erc20(u32, H160),
}

pub struct Pool {
    asset_0: AssetId,
    reserve_0: u128,
    asset_1: AssetId,
    reserve_1: u128,
    fee: u128,
    total_shares: u128,
    shares: HashMap<Principal, u128>,
}

impl Pool {
    pub fn add_liquidity(&mut self, depositor: Principal, amount_0: u128, amount_1: u128) -> u128 {
        // Update pool balances
        self.reserve_0 += amount_0;
        self.reserve_1 += amount_1;

        // Calculate shares to mint
        let shares_minted = self.calculate_shares(amount_0, amount_0);

        // Update total shares and assign to the depositor
        self.total_shares += shares_minted;
        self.shares.entry(depositor).and_modify(|e| *e += shares_minted).or_insert(shares_minted);

        shares_minted
    }

    pub fn remove_liquidity(&mut self, withdrawer: Principal, shares: u128) -> Result<(u128, u128), String> {
        let user_shares = self.shares.get(&withdrawer).ok_or("No shares found for user")?;
        if *user_shares < shares {
            return Err("Insufficient shares".to_string());
        }

        // Calculate the amount of each asset to return
        let amount_0_to_withdraw = self.reserve_0 * shares / self.total_shares;
        let amount_1_to_withdraw = self.reserve_1 * shares / self.total_shares;

        // Adjust the pool balances
        self.reserve_0 -= amount_0_to_withdraw;
        self.reserve_1 -= amount_1_to_withdraw;

        // Burn the shares
        self.total_shares -= shares;
        self.shares.entry(withdrawer).and_modify(|e| *e -= shares);

        Ok((amount_0_to_withdraw, amount_1_to_withdraw))
    }

    pub fn swap(&mut self, asset_in: AssetId, amount_in: u128) -> Result<u128, String> {
        assert!(self.asset_0 == asset_in || self.asset_1 == asset_in, "Invalid asset_in");

        // Check pool balance
        let (reserve_in, reserve_out) = if asset_in == self.asset_0 {
            (self.reserve_0, self.reserve_1)
        } else {
            (self.reserve_1, self.reserve_0)
        };

        // Calculate output amount with fee
        let amount_out_after_fee = self.calculate_output_amount_with_fee(amount_in, reserve_in, reserve_out);

        // Update pool balances
        if asset_in == self.asset_0 {
            self.reserve_0 += amount_in;
            self.reserve_1 -= amount_out_after_fee;
        } else {
            self.reserve_1 += amount_in;
            self.reserve_0 -= amount_out_after_fee;
        }

        Ok(amount_out_after_fee)
    }

    pub fn get_shares(&self, user: Principal) -> u128 {
        *self.shares.get(&user).unwrap_or(&0)
    }

    fn calculate_shares(&self, amount_a: u128, amount_b: u128) -> u128 {
        // Implement logic to calculate shares based on the amounts deposited
        // Example: use a geometric mean or another formula for simplicity
        (amount_a * amount_b).sqrt() as u128
    }

    fn calculate_output_amount_with_fee(&self, amount_in: u128, reserve_in: u128, reserve_out: u128) -> u128 {
        // Implement logic to calculate output amount using a constant product formula
        // Example: (x * y = k) where x and y are asset reserves
        let amount_in_with_fee = amount_in * (10_000 - self.fee) / 10_000;
        let new_reserve_in = reserve_in + amount_in_with_fee;
        let new_reserve_out = (reserve_out * reserve_in) / new_reserve_in;
        reserve_out - new_reserve_out
    }
}

#[cfg(test)]
mod tests {
    use candid::types::principal;

    use super::*;

    #[test]
    fn test_add_liquidity() {
        let mut pool = Pool {
            asset_0: AssetId::Eth(1),
            reserve_0: 0,
            asset_1: AssetId::Erc20(1, H160::zero()),
            reserve_1: 0,
            fee: 0,
            total_shares: 0,
            shares: HashMap::new(),
        };
        let principal_1 = Principal::from_slice(&[1; 29]);
        let shares = pool.add_liquidity(principal_1, 100_000_000, 100_000_000);
        assert_eq!(shares, 100_000_000);
        assert_eq!(pool.reserve_0, 100_000_000);
        assert_eq!(pool.reserve_1, 100_000_000);
        assert_eq!(pool.total_shares, 100_000_000);
        assert_eq!(pool.shares.len(), 1);
        assert_eq!(pool.shares.values().sum::<u128>(), 100_000_000);

        // let principal_2 = Principal::from_slice(&[2; 29]);
        let result = pool.swap(AssetId::Eth(1), 500);
        let amount_out = result.expect("Swap failed");
        println!("Amount out: {}", amount_out);
        assert!(amount_out == 500);

    }

}
