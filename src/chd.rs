pub fn displace(h1: u32, h2: u32, d0: u32, d1: u32) -> u32 {
    // Then, for each bucket Bi, 0 ≤ i < r,
    // we will assign a pair of displacements (d0, d1) so that each key x ∈ Bi is placed in an empty bin
    // given by (f1(x) + d0f2(x) + d1) mod m.

    h1.wrapping_add(d0.wrapping_mul(h2)).wrapping_add(d1)
}
