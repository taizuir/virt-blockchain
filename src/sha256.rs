const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn pad(data: &[u8]) -> Vec<u8> {
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    msg
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let msg = pad(data);
    let mut h = H0;

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn padding_limit(){
        let p55= "a".repeat(55);
        let p56 = "a".repeat(56);
        assert_eq!(sha256_hex(p55.as_bytes()),"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
        assert_eq!(sha256_hex(p56.as_bytes()),"9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318");
        
    }
    #[test]
    fn limit_padding_plus_block(){
        let p63= "a".repeat(63);
        let p64 = "a".repeat(64);
        assert_eq!(sha256_hex(p63.as_bytes()),"7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34");
        assert_eq!(sha256_hex(p64.as_bytes()),"ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
        
    }
    #[test]
    fn limit_padding_plus_several_block(){
        let p119= "a".repeat(119);
        assert_eq!(sha256_hex(p119.as_bytes()),"31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb");
        
    }
    #[test] 
    fn big_input(){
        let p1000000="a".repeat(1000000);
        assert_eq!(sha256_hex(p1000000.as_bytes()),"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0")
    }

    #[test]
    fn same_input_same_output(){
        let in1="qsdf";
        let in2="qsdf";
        assert_eq!(sha256_hex(in1),sha256_hex(in2))
    }
    #[test]
    fn compare_sha256_with_sha256_hex(){
        let input= "qsdf";
        assert_eq!(hex(sha256(input),sha256_hex(input))
    }

    fn is_valid_digest(s: &str) -> bool {
        s.len() == 64
            && s.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }

    #[test]
    fn helper_rejects_bad_digests() {
        assert!(is_valid_digest(&"a".repeat(64)));
        assert!(!is_valid_digest(&"a".repeat(63)));
        assert!(!is_valid_digest(&"a".repeat(65)));
        assert!(!is_valid_digest(&"A".repeat(64)));
        assert!(!is_valid_digest(&"g".repeat(64)));
    }

    #[test]
    fn nist_two_blocks() {
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn your_own_strings() {
        assert_eq!(
            sha256_hex(b"qsdf"),
            "669831a3180f1e77e9e3c904b76d625403924303118ff97acff2d8599b9dc91b"
        );
        assert_eq!(
            sha256_hex(b"hello blockchain"),
            "cf55026ba78c889dbdaf0c32701cdb4d662f3d3ea4460110d3ed2edd0d753e72"
        );
    }

    #[test]
    fn format_of_the_output() {
        for n in [0, 1, 55, 56, 63, 64, 65, 200] {
            let input = "a".repeat(n);
            let digest = sha256_hex(input.as_bytes());
            assert!(
                is_valid_digest(&digest),
                "format invalide pour n={} : {}",
                n,
                digest
            );
        }
    }

    #[test]
    fn pad_layout() {
        for n in [0usize, 1, 55, 56, 63, 64, 100] {
            let data = vec![7u8; n];
            let padded = pad(&data);
            assert_eq!(padded.len() % 64, 0, "n={}", n);
            assert_eq!(&padded[..n], &data[..], "n={}", n);
            assert_eq!(padded[n], 0x80, "n={}", n);
            let tail: [u8; 8] = padded[padded.len() - 8..].try_into().unwrap();
            assert_eq!(u64::from_be_bytes(tail), (n as u64) * 8, "n={}", n);
        }
    }

    #[test]
    fn avalanche() {
        let a = sha256(b"abc");
        let b = sha256(b"abd");
        assert_ne!(a, b);
        let differing_bits: u32 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum();
        assert!(differing_bits > 64, "seulement {} bits différents", differing_bits);
    }
}
