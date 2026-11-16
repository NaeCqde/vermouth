//! A freed copy of a secret must not stay in the process (Linux: the heap is read from /proc/self/mem).
#![cfg(target_os = "linux")]

use std::io::{Read, Seek, SeekFrom};

#[global_allocator]
static ALLOC: vermouth_common::ZeroOnFree = vermouth_common::ZeroOnFree;

/// Counts the copies of `needle` in the heap and in anonymous memory (not the stack, where the needle itself lives).
fn copies_of(needle: &[u8]) -> usize {
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
    let mut mem = std::fs::File::open("/proc/self/mem").unwrap();
    let mut found = 0;
    for line in maps.lines() {
        let mut it = line.split_whitespace();
        let range = it.next().unwrap();
        let perms = it.next().unwrap();
        let path = it.nth(3).unwrap_or("");
        if !perms.starts_with("rw") || path == "[stack]" || path.starts_with("[v") || (path.starts_with('/') && !path.starts_with("/dev/zero")) {
            continue;
        }
        let (a, b) = range.split_once('-').unwrap();
        let (a, b) = (u64::from_str_radix(a, 16).unwrap(), u64::from_str_radix(b, 16).unwrap());
        let mut buf = vec![0u8; (b - a) as usize];
        if mem.seek(SeekFrom::Start(a)).is_err() || mem.read_exact(&mut buf).is_err() {
            continue;
        }
        found += buf.windows(needle.len()).filter(|w| *w == needle).count();
        // `buf` is freed (and wiped) here, so it never counts as a copy
    }
    found
}

fn secret() -> String {
    // built at run time so the literal is not in the binary
    ["very-secret-", "container-", "password-", "0123456789"].concat()
}

#[test]
fn freed_copies_are_wiped() {
    let mut needle = [0u8; 41]; // on the stack: not scanned, not a heap copy
    needle.copy_from_slice(secret().as_bytes());
    let needle = &needle[..];

    // the needle itself sits in this thread's (anonymous) stack: that is the baseline, not a leak
    let base = copies_of(needle);

    let live = secret();
    assert!(copies_of(needle) > base, "the scan must see a live copy (control)");
    drop(live);

    // copies made on the way: formatting, growing a buffer (realloc), cloning
    let mut grown = String::new();
    for _ in 0..40 {
        grown.push_str(&secret());
        grown.push('\n');
    }
    let formatted = format!("{}\n", secret());
    let cloned = formatted.clone();
    assert!(copies_of(needle) >= base + 41);
    drop((grown, formatted, cloned));

    assert_eq!(copies_of(needle), base, "a freed copy of the secret is still in memory");
}
