macro_rules! bigint_impl {
    ($name:ident, $num_limbs:expr) => {
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Hash)]
        pub struct $name(pub [u64; $num_limbs]);

        impl $name {
            pub const fn new(value: [u64; $num_limbs]) -> Self {
                $name(value)
            }
        }

        impl BigInteger for $name {
            const NUM_LIMBS: usize = $num_limbs;

            #[inline]
            fn add_nocarry(&mut self, other: &Self) -> bool {
                let mut carry = 0;

                for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
                    *a = arithmetic::adc(*a, *b, &mut carry);
                }

                carry != 0
            }

            #[inline]
            fn sub_noborrow(&mut self, other: &Self) -> bool {
                let mut borrow = 0;

                for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
                    *a = arithmetic::sbb(*a, *b, &mut borrow);
                }

                borrow != 0
            }

            #[inline]
            fn mul2(&mut self) {
                let mut last = 0;
                for i in &mut self.0 {
                    let tmp = *i >> 63;
                    *i <<= 1;
                    *i |= last;
                    last = tmp;
                }
            }

            #[inline]
            fn muln(&mut self, mut n: u32) {
                if n >= 64 * $num_limbs {
                    *self = Self::from(0);
                    return;
                }

                while n >= 64 {
                    let mut t = 0;
                    for i in &mut self.0 {
                        core::mem::swap(&mut t, i);
                    }
                    n -= 64;
                }

                if n > 0 {
                    let mut t = 0;
                    for i in &mut self.0 {
                        let t2 = *i >> (64 - n);
                        *i <<= n;
                        *i |= t;
                        t = t2;
                    }
                }
            }

            #[inline]
            fn div2(&mut self) {
                let mut t = 0;
                for i in self.0.iter_mut().rev() {
                    let t2 = *i << 63;
                    *i >>= 1;
                    *i |= t;
                    t = t2;
                }
            }

            #[inline]
            fn divn(&mut self, mut n: u32) {
                if n >= 64 * $num_limbs {
                    *self = Self::from(0);
                    return;
                }

                while n >= 64 {
                    let mut t = 0;
                    for i in self.0.iter_mut().rev() {
                        core::mem::swap(&mut t, i);
                    }
                    n -= 64;
                }

                if n > 0 {
                    let mut t = 0;
                    for i in self.0.iter_mut().rev() {
                        let t2 = *i << (64 - n);
                        *i >>= n;
                        *i |= t;
                        t = t2;
                    }
                }
            }

            #[inline]
            fn is_odd(&self) -> bool {
                self.0[0] & 1 == 1
            }

            #[inline]
            fn is_even(&self) -> bool {
                !self.is_odd()
            }

            #[inline]
            fn is_zero(&self) -> bool {
                self.0.iter().all(|&e| e == 0)
            }

            #[inline]
            fn num_bits(&self) -> u32 {
                let mut ret = $num_limbs * 64;
                for i in self.0.iter().rev() {
                    let leading = i.leading_zeros();
                    ret -= leading;
                    if leading != 64 {
                        break;
                    }
                }

                ret
            }

            #[inline]
            fn get_bit(&self, i: usize) -> bool {
                if i >= 64 * $num_limbs {
                    false
                } else {
                    let limb = i / 64;
                    let bit = i - (64 * limb);
                    (self.0[limb] & (1 << bit)) != 0
                }
            }

            #[inline]
            fn from_bits(bits: &[bool]) -> Self {
                let mut res = Self::default();
                let mut acc: u64 = 0;

                let mut bits = bits.to_vec();
                bits.reverse();
                for (i, bits64) in bits.chunks(64).enumerate() {
                    for bit in bits64.iter().rev() {
                        acc <<= 1;
                        acc += *bit as u64;
                    }
                    res.0[i] = acc;
                    acc = 0;
                }
                res
            }

            #[inline]
            fn to_bits(&self) -> Vec<bool> {
                let mut res = Vec::with_capacity(256);
                for b in BitIteratorBE::new(self.0) {
                    res.push(b);
                }
                res
            }

            #[inline]
            fn find_wnaf(&self) -> Vec<i64> {
                let mut res = vec![];

                let mut e = self.clone();
                while !e.is_zero() {
                    let z: i64;
                    if e.is_odd() {
                        z = 2 - (e.0[0] % 4) as i64;
                        if z >= 0 {
                            e.sub_noborrow(&Self::from(z as u64));
                        } else {
                            e.add_nocarry(&Self::from((-z) as u64));
                        }
                    } else {
                        z = 0;
                    }
                    res.push(z);
                    e.div2();
                }

                res
            }

            #[inline]
            fn mul_no_reduce(this: &[u64], other: &[u64]) -> Self {
                assert!(this.len() == $num_limbs / 2);
                assert!(other.len() == $num_limbs / 2);

                let mut r = [0u64; $num_limbs];
                for i in 0..$num_limbs / 2 {
                    let mut carry = 0u64;
                    for j in 0..$num_limbs / 2 {
                        r[j + i] =
                            arithmetic::mac_with_carry(r[j + i], this[i], other[j], &mut carry);
                    }
                    r[$num_limbs / 2 + i] = carry;
                }
                Self::new(r)
            }

            #[inline]
            fn mul_no_reduce_lo(this: &[u64], other: &[u64]) -> Self {
                assert!(this.len() == $num_limbs);
                assert!(other.len() == $num_limbs);

                let mut r = [0u64; $num_limbs];
                for i in 0..$num_limbs {
                    let mut carry = 0u64;
                    for j in 0..($num_limbs - i) {
                        r[j + i] =
                            arithmetic::mac_with_carry(r[j + i], this[i], other[j], &mut carry);
                    }
                }
                Self::new(r)
            }

            #[inline]
            fn from_slice(slice: &[u64]) -> Self {
                let mut repr = Self::default();
                for (limb, &value) in repr.0.iter_mut().zip(slice) {
                    *limb = value;
                }
                repr
            }
        }

        impl CanonicalSerialize for $name {
            #[inline]
            fn serialize<W: Write>(&self, writer: W) -> Result<(), SerializationError> {
                self.write(writer)?;
                Ok(())
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                Self::SERIALIZED_SIZE
            }
        }

        impl ConstantSerializedSize for $name {
            const SERIALIZED_SIZE: usize = Self::NUM_LIMBS * 8;
            const UNCOMPRESSED_SIZE: usize = Self::SERIALIZED_SIZE;
        }

        impl CanonicalDeserialize for $name {
            #[inline]
            fn deserialize<R: Read>(reader: R) -> Result<Self, SerializationError> {
                let value = Self::read(reader)?;
                Ok(value)
            }
        }

        impl ToBytes for $name {
            #[inline]
            fn write<W: Write>(&self, writer: W) -> IoResult<()> {
                self.0.write(writer)
            }
        }

        impl FromBytes for $name {
            #[inline]
            fn read<R: Read>(reader: R) -> IoResult<Self> {
                <[u64; $num_limbs]>::read(reader).map(Self::new)
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                for i in self.0.iter().rev() {
                    write!(f, "{:016X}", *i)?;
                }
                Ok(())
            }
        }

        impl Ord for $name {
            #[inline]
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                for (a, b) in self.0.iter().rev().zip(other.0.iter().rev()) {
                    if a < b {
                        return core::cmp::Ordering::Less;
                    } else if a > b {
                        return core::cmp::Ordering::Greater;
                    }
                }

                core::cmp::Ordering::Equal
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Distribution<$name> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> $name {
                $name(rng.gen())
            }
        }

        impl AsMut<[u64]> for $name {
            #[inline]
            fn as_mut(&mut self) -> &mut [u64] {
                &mut self.0
            }
        }

        impl AsRef<[u64]> for $name {
            #[inline]
            fn as_ref(&self) -> &[u64] {
                &self.0
            }
        }

        impl From<u64> for $name {
            #[inline]
            fn from(val: u64) -> $name {
                let mut repr = Self::default();
                repr.0[0] = val;
                repr
            }
        }
    };
}
