use uint::construct_uint;

construct_uint! {
  /// 256-bit unsigned integer.
  pub struct u256(4);
}

/*
 formula for calculating amount out with 0.3% fee;
 Yout = Y x Xi x 997 / (X * 1000 + Xi * 997)
*/
pub fn get_amount_out(amount_in: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    let amount_in_with_fee = u256::from(amount_in) * u256::from(997);
    let numerator = amount_in_with_fee * u256::from(reserve_out);
    let denominator = u256::from(reserve_in) * u256::from(1000) + amount_in_with_fee;
    let amount_out = numerator / denominator;
    amount_out.as_u128()
}
