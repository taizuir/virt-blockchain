use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use virt_blockchain::{sha256::sha256_hex, Blockchain};

// cargo test runs tests in this file on separate threads by default; since
// the global allocator below counts bytes for the whole process, concurrent
// tests would pollute each other's measurements. This mutex serializes them.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn serialized() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// Wraps the system allocator to track live (allocated - freed) bytes for this
// test binary, so the memory tests can measure real heap usage without a crate.
struct CountingAllocator;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn live_bytes() -> usize {
    LIVE_BYTES.load(Ordering::SeqCst)
}

// ---- correctness ----

#[test]
fn genesis_block_satisfies_difficulty_and_chain_is_valid() {
    let _guard = serialized();
    let bc = Blockchain::new();
    assert!(bc.access_last().hash.starts_with('0'));
    assert!(bc.is_valid());
}

#[test]
fn adding_blocks_keeps_chain_linked_and_valid() {
    let _guard = serialized();
    let mut bc = Blockchain::new();
    bc.add_block("first".into());
    bc.add_block("second".into());

    assert_eq!(bc.chain.len(), 3);
    for i in 0..bc.chain.len() {
        assert_eq!(bc.chain[i].index, i as u64);
    }
    for i in 1..bc.chain.len() {
        assert_eq!(bc.chain[i].prev_hash, bc.chain[i - 1].hash);
    }
    assert!(bc.is_valid());
}

#[test]
fn tampering_with_data_breaks_validity() {
    let _guard = serialized();
    let mut bc = Blockchain::new();
    bc.add_block("payload".into());

    bc.chain[1].data = "forged payload".into();

    assert!(!bc.is_valid());
}

#[test]
fn tampering_with_prev_hash_breaks_validity() {
    let _guard = serialized();
    let mut bc = Blockchain::new();
    bc.add_block("a".into());
    bc.add_block("b".into());

    bc.chain[2].prev_hash = "0".repeat(64);

    assert!(!bc.is_valid());
}

#[test]
fn mined_hash_matches_recomputed_hash() {
    let _guard = serialized();
    let mut bc = Blockchain::new();
    bc.add_block("check".into());

    let last = bc.access_last();
    assert_eq!(last.hash, last.calculate_hash());
}

// ---- speed ----

#[test]
fn raw_sha256_throughput() {
    let _guard = serialized();
    let input = b"the quick brown fox jumps over the lazy dog";
    let iterations = 50_000;

    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(sha256_hex(input));
    }
    let elapsed = start.elapsed();

    let hashes_per_sec = iterations as f64 / elapsed.as_secs_f64();
    println!(
        "raw sha256: {iterations} hashes in {:?} ({:.0} hashes/sec)",
        elapsed, hashes_per_sec
    );

    // Sanity bound only — not a strict perf gate, just catches something
    // going pathologically wrong (e.g. accidental O(n^2) behavior).
    assert!(elapsed.as_secs() < 10);
}

#[test]
fn mining_speed_by_difficulty() {
    let _guard = serialized();
    for difficulty in 1..=3 {
        let start = Instant::now();
        let bc = virt_blockchain::Blockchain::with_difficulty(difficulty);
        let elapsed = start.elapsed();

        let nonce = bc.access_last().nonce;
        let hashes_per_sec = (nonce + 1) as f64 / elapsed.as_secs_f64().max(1e-9);
        println!(
            "difficulty {difficulty}: nonce={nonce} elapsed={:?} (~{:.0} hashes/sec)",
            elapsed, hashes_per_sec
        );

        assert!(bc.is_valid());
    }
}

#[test]
#[ignore = "difficulty 5 averages ~1M hash attempts; run explicitly with --ignored, ideally --release"]
fn mining_speed_difficulty_5() {
    let _guard = serialized();
    let start = Instant::now();
    let bc = virt_blockchain::Blockchain::with_difficulty(5);
    let elapsed = start.elapsed();

    let nonce = bc.access_last().nonce;
    println!(
        "difficulty 5: nonce={nonce} elapsed={:?} (~{:.0} hashes/sec)",
        elapsed,
        (nonce + 1) as f64 / elapsed.as_secs_f64().max(1e-9)
    );

    assert!(bc.is_valid());
}

// ---- memory ----

#[test]
fn stack_size_of_block() {
    let _guard = serialized();
    let size = std::mem::size_of::<virt_blockchain::Block>();
    println!("size_of::<Block>() = {size} bytes");
    assert!(size < 200, "Block grew unexpectedly large ({size} bytes)");
}

#[test]
fn heap_usage_scales_with_chain_length_and_is_freed_on_drop() {
    let _guard = serialized();
    let baseline = live_bytes();

    let mut bc = Blockchain::new();
    for i in 0..20 {
        bc.add_block(format!("block {i}"));
    }

    let with_chain = live_bytes();
    let growth = with_chain.saturating_sub(baseline);
    println!(
        "heap growth for 21 blocks: {growth} bytes (~{:.1} bytes/block)",
        growth as f64 / 21.0
    );
    assert!(growth > 0, "expected heap usage to grow as blocks are added");

    drop(bc);
    let after_drop = live_bytes();
    let freed = with_chain.saturating_sub(after_drop);
    println!(
        "heap after drop: {after_drop} bytes (baseline was {baseline}, freed {freed} of {growth})"
    );
    // Allow slack: unrelated allocations (e.g. from println! itself) happen
    // between measurements, so this isn't a byte-exact check — just confirms
    // the chain's memory was actually released rather than leaked.
    assert!(
        freed as f64 >= growth as f64 * 0.8,
        "chain's memory doesn't look released after drop: freed {freed} of {growth} bytes"
    );
}
