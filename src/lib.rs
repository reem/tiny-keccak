//! An implementation of the FIPS-202-defined SHA-3 and SHAKE functions.
//!
//! The `Keccak-f[1600]` permutation is fully unrolled; it's nearly as fast
//! as the Keccak team's optimized permutation.
//!
//! ## Building
//!
//! ```bash
//! cargo build
//! ```
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tiny-keccak = "0.1"
//! ```
//!
//! and this to your crate root:
//!
//! ```rust
//! extern crate tiny_keccak;
//! ```
//!
//! Original implemntation in C: 
//! https://github.com/coruus/keccak-tiny
//!
//! Implementor: David Leon Gil
//! 
//! Port to rust: 
//! Marek Kotewicz (marek.kotewicz@gmail.com)
//!
//! License: CC0, attribution kindly requested. Blame taken too,
//! but not liability.

const RHO: [u32; 24] = [
	 1,  3,  6, 10, 15, 21,
	28, 36, 45, 55,  2, 14,
	27, 41, 56,  8, 25, 43,
	62, 18, 39, 61, 20, 44
];

const PI: [usize; 24] = [
	10,  7, 11, 17, 18, 3,
	 5, 16,  8, 21, 24, 4,
	15, 23, 19, 13, 12, 2,
	20, 14, 22,  9,  6, 1
];

const RC: [u64; 24] = [
	1u64, 0x8082u64, 0x800000000000808au64, 0x8000000080008000u64,
	0x808bu64, 0x80000001u64, 0x8000000080008081u64, 0x8000000000008009u64,
	0x8au64, 0x88u64, 0x80008009u64, 0x8000000au64,
	0x8000808bu64, 0x800000000000008bu64, 0x8000000000008089u64, 0x8000000000008003u64,
	0x8000000000008002u64, 0x8000000000000080u64, 0x800au64, 0x800000008000000au64,
	0x8000000080008081u64, 0x8000000000008080u64, 0x80000001u64, 0x8000000080008008u64
];

macro_rules! REPEAT4 {
	($e: expr) => ( $e; $e; $e; $e; )
}

macro_rules! REPEAT5 {
	($e: expr) => ( $e; $e; $e; $e; $e; )
}

macro_rules! REPEAT6 {
	($e: expr) => ( $e; $e; $e; $e; $e; $e; )
}

macro_rules! REPEAT24 {
	($e: expr, $s: expr) => ( 
		REPEAT6!({ $e; $s; }); 
		REPEAT6!({ $e; $s; }); 
		REPEAT6!({ $e; $s; }); 
		REPEAT5!({ $e; $s; }); 
		$e;
	)
}

macro_rules! FOR5 {
	($v: expr, $s: expr, $e: expr) => { 
		$v = 0; 
		REPEAT4!({
			$e;
			$v += $s;
		});
		$e;
	}
}

/// keccak-f[1600]
pub fn keccakf(a: &mut [u64]) {
	unsafe {
		let mut b: [u64; 5] = [0; 5];
		let mut t: u64;
		let mut x: usize;
		let mut y: usize;

		for i in 0..24 {
			// Theta
			FOR5!(x, 1, {
				*b.get_unchecked_mut(x) = 0;
				FOR5!(y, 5, {
					*b.get_unchecked_mut(x) ^= *a.get_unchecked(x + y);
				});
			});

			FOR5!(x, 1, {
				FOR5!(y, 5, {
					*a.get_unchecked_mut(y + x) ^= *b.get_unchecked((x + 4) % 5) ^ b.get_unchecked((x + 1) % 5).rotate_left(1);
				});
			});

			// Rho and pi
			t = *a.get_unchecked(1); 
			x = 0;
			REPEAT24!({
				*b.get_unchecked_mut(0) = *a.get_unchecked(*PI.get_unchecked(x));
				*a.get_unchecked_mut(*PI.get_unchecked(x)) = t.rotate_left(*RHO.get_unchecked(x));
			}, {
				t = *b.get_unchecked(0);
				x += 1;
			});

			// Chi
			FOR5!(y, 5, {
				FOR5!(x, 1, {
					*b.get_unchecked_mut(x) = *a.get_unchecked(y + x);
				});
				FOR5!(x, 1, {
					*a.get_unchecked_mut(y + x) = *b.get_unchecked(x) ^ ((!b.get_unchecked((x + 1) % 5)) & b.get_unchecked((x + 2) % 5));
				});
			});

			// Iota
			*a.get_unchecked_mut(0) ^= *RC.get_unchecked(i);
		}
	}
}

fn xorin(dst: &mut [u8], src: &[u8], len: usize) {
	unsafe {
		for i in 0..len {
			*dst.get_unchecked_mut(i) ^= *src.get_unchecked(i);
		}
	}
}

fn setout(src: &[u8], dst: &mut [u8], len: usize) {
	unsafe {
		::std::ptr::copy(src.as_ptr(), dst.as_mut_ptr(), len);
	}
}

/// Total number of lanes.
const PLEN: usize = 25;

/// Lets cheat borrow checker. 
fn as_bytes_slice<'a, 'b>(ints: &'a [u64]) -> &'b [u8] {
	unsafe {
		::std::slice::from_raw_parts(ints.as_ptr() as *mut u8, ints.len() * 8)
	}
}

/// Lets cheat borrow checker... again.
fn as_mut_bytes_slice<'a, 'b>(ints: &'a mut [u64]) -> &'b mut [u8] {
	unsafe {
		::std::slice::from_raw_parts_mut(ints.as_mut_ptr() as *mut u8, ints.len() * 8)
	}
}

/// This structure should be used to create keccak/sha3 hash.
///
/// ```rust
/// extern crate tiny_keccak;
/// use tiny_keccak::Keccak;
/// 
/// fn main() {
/// 	let mut sha3 = Keccak::new_sha3_256();
/// 	let data: Vec<u8> = From::from("hello");
/// 	let data2: Vec<u8> = From::from("world");
/// 	
/// 	sha3.update(&data);
/// 	sha3.update(&[b' ']);
/// 	sha3.update(&data2);
///
/// 	let mut res: [u8; 32] = [0; 32];
/// 	sha3.finalize(&mut res);
///
/// 	let expected = vec![
/// 		0x64, 0x4b, 0xcc, 0x7e, 0x56, 0x43, 0x73, 0x04,
/// 		0x09, 0x99, 0xaa, 0xc8, 0x9e, 0x76, 0x22, 0xf3,
/// 		0xca, 0x71, 0xfb, 0xa1, 0xd9, 0x72, 0xfd, 0x94,
/// 		0xa3, 0x1c, 0x3b, 0xfb, 0xf2, 0x4e, 0x39, 0x38
/// 	];
///
/// 	let ref_ex: &[u8] = &expected;
/// 	assert_eq!(&res, ref_ex);
/// }
/// ```
pub struct Keccak {
	a: [u64; PLEN],
	offset: usize,
	rate: usize,
	delim: u8
}

impl Clone for Keccak {
	fn clone(&self) -> Self {
		use std::mem;
		use std::ptr;

		unsafe {
			let mut res: Keccak = mem::uninitialized();
			ptr::copy(self.a.as_ptr(), res.a.as_mut_ptr(), self.a.len());
			res.offset = self.offset;
			res.rate = self.rate;
			res.delim = self.delim;
			res
		}
	}
}

macro_rules! impl_constructor {
	($name: ident, $bits: expr, $delim: expr) => {
		pub fn $name() -> Keccak {
			Keccak::new(200 - $bits/4, $delim)
		}
	}
}

impl Keccak {
	fn new(rate: usize, delim: u8) -> Keccak {
		Keccak {
			a: [0; PLEN],
			offset: 0,
			rate: rate,
			delim: delim
		}
	}

	impl_constructor!(new_shake128,  128, 0x1f);
	impl_constructor!(new_shake256,  256, 0x1f);
	impl_constructor!(new_keccak224, 224, 0x01);
	impl_constructor!(new_keccak256, 256, 0x01);
	impl_constructor!(new_keccak384, 384, 0x01);
	impl_constructor!(new_keccak512, 512, 0x01);
	impl_constructor!(new_sha3_224,  224, 0x06);
	impl_constructor!(new_sha3_256,  256, 0x06);
	impl_constructor!(new_sha3_384,  384, 0x06);
	impl_constructor!(new_sha3_512,  512, 0x06);

	pub fn update(&mut self, input: &[u8]) {
		self.absorb(input);
	}

	pub fn finalize(mut self, output: &mut [u8]) {
		self.pad();
		
		// apply keccakf
		keccakf(&mut self.a);

		// squeeze output
		self.squeeze(output);
	}

	// Absorb input
	fn absorb(&mut self, input: &[u8]) {
		let mut a = as_mut_bytes_slice(&mut self.a);

		let inlen = input.len();
		let mut rate = self.rate - self.offset;

		//first foldp
		let mut ip = 0;
		let mut l = inlen;
		while l >= rate {
			xorin(&mut a[self.offset..], &input[ip..], rate);
			keccakf(&mut self.a);
			ip += rate;
			l -= rate;
			rate = self.rate;
			self.offset = 0;
		}

		// Xor in the last block 
		xorin(&mut a[self.offset..], &input[ip..], l);
		self.offset += l;
	}

	fn pad(&mut self) {
		let mut a = as_mut_bytes_slice(&mut self.a);

		let offset = self.offset;
		let rate = self.rate;

		unsafe {
			*a.get_unchecked_mut(offset) ^= self.delim;
			*a.get_unchecked_mut(rate - 1) ^= 0x80;
		}
	}

	// squeeze output
	fn squeeze(&mut self, output: &mut [u8]) {
		let a = as_bytes_slice(&mut self.a);

		let outlen = output.len();
		let rate = self.rate;

		// second foldp
		let mut op = 0;
		let mut l = outlen;
		while l >= rate {
			setout(&a, &mut output[op..], rate);
			keccakf(&mut self.a);
			op += rate;
			l -= rate;
		}

		setout(&a, &mut output[op..], l);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_keccak() {
		let keccak = Keccak::new_keccak256();
		let mut res: [u8; 32] = [0; 32];
		keccak.finalize(&mut res);

		let expected = vec![
			0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
			0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
			0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
			0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70
		];

		let ref_ex: &[u8] = &expected;
		assert_eq!(&res, ref_ex);
	}

	#[test]
	fn empty_sha3_256() {
		let sha3 = Keccak::new_sha3_256();
		let mut res: [u8; 32] = [0; 32];
		sha3.finalize(&mut res);

		let expected = vec![
			0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66,
			0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61, 0xd6, 0x62,
			0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 
			0x82, 0xd8, 0x0a, 0x4b, 0x80, 0xf8, 0x43, 0x4a
		];

		let ref_ex: &[u8] = &expected;
		assert_eq!(&res, ref_ex);
	}

	#[test]
	fn string_sha3_256() {
		let mut sha3 = Keccak::new_sha3_256();
		let data: Vec<u8> = From::from("hello");
		sha3.update(&data);

		let mut res: [u8; 32] = [0; 32];
		sha3.finalize(&mut res);

		let expected = vec![
			0x33, 0x38, 0xbe, 0x69, 0x4f, 0x50, 0xc5, 0xf3,
			0x38, 0x81, 0x49, 0x86, 0xcd, 0xf0, 0x68, 0x64, 
			0x53, 0xa8, 0x88, 0xb8, 0x4f, 0x42, 0x4d, 0x79,
			0x2a, 0xf4, 0xb9, 0x20, 0x23, 0x98, 0xf3, 0x92
		];

		let ref_ex: &[u8] = &expected;
		assert_eq!(&res, ref_ex);
	}

	#[test]
	fn string_sha3_256_parts() {
		let mut sha3 = Keccak::new_sha3_256();
		let data: Vec<u8> = From::from("hell");
		sha3.update(&data);
		sha3.update(&[b'o']);

		let mut res: [u8; 32] = [0; 32];
		sha3.finalize(&mut res);

		let expected = vec![
			0x33, 0x38, 0xbe, 0x69, 0x4f, 0x50, 0xc5, 0xf3,
			0x38, 0x81, 0x49, 0x86, 0xcd, 0xf0, 0x68, 0x64, 
			0x53, 0xa8, 0x88, 0xb8, 0x4f, 0x42, 0x4d, 0x79,
			0x2a, 0xf4, 0xb9, 0x20, 0x23, 0x98, 0xf3, 0x92
		];

		let ref_ex: &[u8] = &expected;
		assert_eq!(&res, ref_ex);
	}

	#[test]
	fn string_sha3_256_parts5() {
		let mut sha3 = Keccak::new_sha3_256();
		sha3.update(&[b'h']);
		sha3.update(&[b'e']);
		sha3.update(&[b'l']);
		sha3.update(&[b'l']);
		sha3.update(&[b'o']);

		let mut res: [u8; 32] = [0; 32];
		sha3.finalize(&mut res);

		let expected = vec![
			0x33, 0x38, 0xbe, 0x69, 0x4f, 0x50, 0xc5, 0xf3,
			0x38, 0x81, 0x49, 0x86, 0xcd, 0xf0, 0x68, 0x64, 
			0x53, 0xa8, 0x88, 0xb8, 0x4f, 0x42, 0x4d, 0x79,
			0x2a, 0xf4, 0xb9, 0x20, 0x23, 0x98, 0xf3, 0x92
		];

		let ref_ex: &[u8] = &expected;
		assert_eq!(&res, ref_ex);
	}

	#[test]
	fn long_string_sha3_512() {
		let mut sha3 = Keccak::new_sha3_512();
		let data: Vec<u8> = From::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.");

		sha3.update(&data);
		let mut res: [u8; 64] = [0; 64];
		sha3.finalize(&mut res);

		let expected = vec![
			0xf3, 0x2a, 0x94, 0x23, 0x55, 0x13, 0x51, 0xdf, 
			0x0a, 0x07, 0xc0, 0xb8, 0xc2, 0x0e, 0xb9, 0x72,
			0x36, 0x7c, 0x39, 0x8d, 0x61, 0x06, 0x60, 0x38,
			0xe1, 0x69, 0x86, 0x44, 0x8e, 0xbf, 0xbc, 0x3d,
			0x15, 0xed, 0xe0, 0xed, 0x36, 0x93, 0xe3, 0x90,
			0x5e, 0x9a, 0x8c, 0x60, 0x1d, 0x9d, 0x00, 0x2a,
			0x06, 0x85, 0x3b, 0x97, 0x97, 0xef, 0x9a, 0xb1,
			0x0c, 0xbd, 0xe1, 0x00, 0x9c, 0x7d, 0x0f, 0x09
		];


		let ref_res: &[u8] = &res;
		let ref_ex: &[u8] = &expected;
		assert_eq!(ref_res, ref_ex);
	}

	#[test]
	fn long_string_sha3_512_parts() {
		let mut sha3 = Keccak::new_sha3_512();
		let data: Vec<u8> = From::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ");
		let data2: Vec<u8> = From::from("ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.");

		sha3.update(&data);
		sha3.update(&data2);

		let mut res: [u8; 64] = [0; 64];
		sha3.finalize(&mut res);

		let expected = vec![
			0xf3, 0x2a, 0x94, 0x23, 0x55, 0x13, 0x51, 0xdf, 
			0x0a, 0x07, 0xc0, 0xb8, 0xc2, 0x0e, 0xb9, 0x72,
			0x36, 0x7c, 0x39, 0x8d, 0x61, 0x06, 0x60, 0x38,
			0xe1, 0x69, 0x86, 0x44, 0x8e, 0xbf, 0xbc, 0x3d,
			0x15, 0xed, 0xe0, 0xed, 0x36, 0x93, 0xe3, 0x90,
			0x5e, 0x9a, 0x8c, 0x60, 0x1d, 0x9d, 0x00, 0x2a,
			0x06, 0x85, 0x3b, 0x97, 0x97, 0xef, 0x9a, 0xb1,
			0x0c, 0xbd, 0xe1, 0x00, 0x9c, 0x7d, 0x0f, 0x09
		];

		let ref_res: &[u8] = &res;
		let ref_ex: &[u8] = &expected;
		assert_eq!(ref_res, ref_ex);
	}
}

