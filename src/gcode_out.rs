//! What every G-code file of the program shares: the one way a number is written (a number that is not finite is refused, never written as `NaN` or `inf` for the controller to
//! choke on) and the standard header with the generator, its version, the units, the file the work came from with its SHA-256, and the tools.

/// A number as the controller reads it: at most three decimals, no trailing zeros, no `-0`. A value that is not finite is an error.
pub fn number(v: f64) -> Result<String, String> {
    if !v.is_finite() {
        return Err(format!(
            "{v} cannot be written in G-code: it is not a finite number"
        ));
    }
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    Ok(if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    })
}

/// The SHA-256 of `data` as 64 lowercase hex digits.
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// What the header says. `source` is the name of the file the work came from and its SHA-256 (none when the work is not made from a file).
pub struct Header<'a> {
    pub generator: &'a str,
    pub version: &'a str,
    pub source: Option<(&'a str, String)>,
    pub tools: &'a str,
}

/// The header as comment lines (a `;` comment is read by the controllers the program is made for, and by the readers of the suite): the generator and its version, the units,
/// the source file with its hash and the tools. A line break inside a name is turned into a space, so a name cannot add a line of G-code.
pub fn header(h: &Header) -> String {
    let one = |s: &str| s.replace(['\r', '\n'], " ");
    let mut out = format!(
        "; generator: {} {}\n; units: mm (G21)\n",
        one(h.generator),
        one(h.version)
    );
    match &h.source {
        Some((name, hash)) => out.push_str(&format!("; source: {} sha256:{hash}\n", one(name))),
        None => out.push_str("; source: none\n"),
    }
    out.push_str(&format!("; tools: {}\n", one(h.tools)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short_and_never_negative_zero() {
        assert_eq!(number(1.0).unwrap(), "1");
        assert_eq!(number(0.30000001).unwrap(), "0.3");
        assert_eq!(number(-0.0001).unwrap(), "0");
        assert_eq!(number(-2.5).unwrap(), "-2.5");
    }

    #[test]
    fn a_number_that_is_not_finite_is_refused() {
        assert!(number(f64::NAN).is_err());
        assert!(number(f64::INFINITY).is_err());
        assert!(number(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn sha256_matches_the_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // two blocks
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn the_header_has_the_generator_the_units_the_source_and_the_tools() {
        let h = header(&Header {
            generator: "g",
            version: "1.2",
            source: Some(("a\nb.png", "ff".into())),
            tools: "laser",
        });
        assert_eq!(
            h,
            "; generator: g 1.2\n; units: mm (G21)\n; source: a b.png sha256:ff\n; tools: laser\n"
        );
        let none = header(&Header {
            generator: "g",
            version: "1",
            source: None,
            tools: "t",
        });
        assert!(none.contains("; source: none\n"));
    }
}
