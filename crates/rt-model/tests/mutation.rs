//! Geçerli bir feed'in küçük mutasyonlarında decoder sözleşmesini sınar.
//!
//! Bu test fuzzing'in yerine geçmez; seed corpus'un çevresindeki sık görülen
//! bozulmaları ucuz ve deterministik biçimde her CI koşumunda tekrarlar.

mod common;

use common::minimal_feed;
use gtfs_rt_model::decode_feed_message;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn assert_total_and_deterministic(bytes: &[u8]) {
    let first = catch_unwind(AssertUnwindSafe(|| decode_feed_message(bytes)))
        .expect("decoder mutated input üzerinde panic yaptı");
    let second = decode_feed_message(bytes);

    assert_eq!(first, second, "aynı mutated input deterministik çözülmedi");
}

#[test]
fn every_truncation_of_valid_feed_is_total() {
    let seed = minimal_feed();

    for end in 0..=seed.len() {
        assert_total_and_deterministic(&seed[..end]);
    }
}

#[test]
fn every_single_bit_flip_of_valid_feed_is_total() {
    let seed = minimal_feed();

    for byte_index in 0..seed.len() {
        for bit in 0..8 {
            let mut mutated = seed.clone();
            mutated[byte_index] ^= 1 << bit;
            assert_total_and_deterministic(&mutated);
        }
    }
}

#[test]
fn common_tail_and_boundary_mutations_are_total() {
    let seed = minimal_feed();
    let mutations: &[&[u8]] = &[
        &[0x00],
        &[0x7b],
        &[0x3c, b'h', b't', b'm', b'l', b'>'],
        &[0x80],
        &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
    ];

    for suffix in mutations {
        let mut appended = seed.clone();
        appended.extend_from_slice(suffix);
        assert_total_and_deterministic(&appended);

        let mut prefixed = suffix.to_vec();
        prefixed.extend_from_slice(&seed);
        assert_total_and_deterministic(&prefixed);
    }
}
