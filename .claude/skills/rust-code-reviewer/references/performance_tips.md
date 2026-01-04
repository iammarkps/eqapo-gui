# Rust Performance Optimization Tips

Guidelines for writing efficient Rust code.

## Memory Allocation

### Minimize Allocations

```rust
// Avoid: Multiple allocations
let mut result = Vec::new();
for i in 0..1000 {
    result.push(i);
}

// Better: Pre-allocate capacity
let mut result = Vec::with_capacity(1000);
for i in 0..1000 {
    result.push(i);
}

// Best: Use iterator
let result: Vec<_> = (0..1000).collect();
```

### String Building

```rust
// Slow: Multiple allocations
let mut s = String::new();
for i in 0..100 {
    s = s + &i.to_string(); // Allocates each time!
}

// Fast: Reuse buffer
let mut s = String::with_capacity(300);
for i in 0..100 {
    use std::fmt::Write;
    write!(&mut s, "{}", i).unwrap();
}

// Or use format! for one-time construction
let s = format!("{} {} {}", a, b, c);
```

### Buffer Reuse

```rust
// Pattern: Reuse buffers in hot loops
let mut buffer = Vec::with_capacity(1024);
for item in items {
    buffer.clear();
    process_into(&mut buffer, item);
    output.write_all(&buffer)?;
}
```

## Iterator Optimization

### Avoid Unnecessary Collections

```rust
// Bad: Intermediate collection
let result: Vec<_> = data
    .iter()
    .map(|x| x * 2)
    .collect::<Vec<_>>()
    .into_iter()
    .filter(|x| x > &10)
    .collect();

// Good: Single iterator chain
let result: Vec<_> = data
    .iter()
    .map(|x| x * 2)
    .filter(|x| x > &10)
    .collect();
```

### Use Iterator Adapters

```rust
// Slow: Manual loop
let mut sum = 0;
for &x in numbers {
    if x > 0 {
        sum += x * 2;
    }
}

// Fast: Iterator chain (often optimizes better)
let sum: i32 = numbers
    .iter()
    .filter(|&&x| x > 0)
    .map(|&x| x * 2)
    .sum();
```

### Parallel Iterators (rayon)

```rust
use rayon::prelude::*;

// Sequential
let sum: i32 = data.iter().map(|x| expensive(x)).sum();

// Parallel (use when work per item is significant)
let sum: i32 = data.par_iter().map(|x| expensive(x)).sum();
```

## Data Structure Selection

### Vec vs VecDeque vs LinkedList

```rust
// Vec: Default choice for sequences
// - Fast random access: O(1)
// - Fast push/pop from end: O(1) amortized
// - Slow insert at front: O(n)

// VecDeque: When you need both ends
// - Fast push/pop from both ends: O(1) amortized
// - Slightly slower than Vec overall

// LinkedList: Almost never (use VecDeque instead)
```

### HashMap vs BTreeMap

```rust
// HashMap: Default for key-value storage
// - Faster lookups: O(1) expected
// - No ordering
// - Good for most use cases

// BTreeMap: When you need ordering
// - Sorted keys
// - Slower lookups: O(log n)
// - Range queries
// - Deterministic iteration order
```

### HashSet vs BTreeSet

```rust
// HashSet: Fast membership testing
// - O(1) expected insert/contains
// - No ordering

// BTreeSet: Ordered set operations
// - O(log n) insert/contains
// - Sorted elements
// - Range operations
```

## Avoiding Copies

### Use References

```rust
// Bad: Copy large struct
fn process(config: Config) { } // Copies Config

// Good: Borrow
fn process(config: &Config) { } // No copy
```

### Cow for Conditional Ownership

```rust
use std::borrow::Cow;

// Only allocate when necessary
fn normalize<'a>(input: &'a str) -> Cow<'a, str> {
    if input.chars().all(|c| c.is_ascii_lowercase()) {
        Cow::Borrowed(input) // No allocation
    } else {
        Cow::Owned(input.to_lowercase()) // Allocate only when needed
    }
}
```

### Move Instead of Clone

```rust
// Bad: Clone when not needed
let data = large_vec.clone();
process_and_consume(data);
// large_vec is still in scope but unused

// Good: Move
let data = large_vec;
process_and_consume(data);
```

## Smart Pointer Performance

### Box vs Rc vs Arc

```rust
// Box: Single owner, heap allocation
// - Smallest overhead
// - Use for large types or recursive structures

// Rc: Shared ownership, single-threaded
// - Reference counting overhead
// - Use when you need shared ownership

// Arc: Shared ownership, multi-threaded
// - Atomic reference counting (slower than Rc)
// - Use only when crossing thread boundaries
```

### Avoid Rc/Arc Cycles

```rust
// Use Weak to break cycles
struct Node {
    value: i32,
    parent: Weak<Node>, // Use Weak, not Rc
    children: Vec<Rc<Node>>,
}
```

## Function Optimization

### Inline Hints

```rust
// Small frequently-called functions
#[inline]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Force inlining (use sparingly)
#[inline(always)]
fn critical_path() { }

// Prevent inlining (for debugging)
#[inline(never)]
fn debug_function() { }
```

### Generic vs Dynamic Dispatch

```rust
// Static dispatch (monomorphization)
// - Faster: no vtable lookup
// - Larger binary: code duplicated for each type
fn process<T: Display>(item: &T) {
    println!("{}", item);
}

// Dynamic dispatch
// - Slower: vtable lookup
// - Smaller binary: single implementation
fn process(item: &dyn Display) {
    println!("{}", item);
}
```

## Numeric Optimization

### Integer Operations

```rust
// Prefer smaller types when range allows
let counter: u8 = 0; // vs i32 for values 0-255

// Use wrapping/saturating math explicitly
let result = value.saturating_add(10);
let result = value.wrapping_sub(5);

// Avoid division when multiplication works
let halved = value >> 1; // Faster than value / 2
```

### Floating Point

```rust
// Avoid comparisons with ==
// Bad
if x == 0.1 { }

// Good
const EPSILON: f64 = 1e-10;
if (x - 0.1).abs() < EPSILON { }

// Use f32 when precision allows (faster)
let value: f32 = 3.14;
```

## Compilation Optimization

### Profile-Guided Optimization

```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization
opt-level = 3           # Maximum optimization
```

### Target-Specific Optimization

```bash
# Build for specific CPU
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Unsafe Optimization

Use unsafe only when profiling shows it's needed.

```rust
// Safety-critical: Bounds checking
let value = slice.get(index).unwrap();

// Performance-critical (after profiling!)
let value = unsafe { *slice.get_unchecked(index) };
// Must prove index < slice.len()
```

## Async Performance

### Spawning Tasks

```rust
// Don't spawn for trivial work
// Bad: Task overhead > work
for item in items {
    tokio::spawn(async move {
        item.process();
    });
}

// Good: Batch work or use rayon
tokio::spawn(async move {
    for item in items {
        item.process();
    }
});
```

### Buffered I/O

```rust
use tokio::io::BufReader;

// Unbuffered: Many small reads
let file = File::open("data.txt").await?;

// Buffered: Fewer system calls
let file = BufReader::new(File::open("data.txt").await?);
```

## Profiling

Always profile before optimizing!

```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph

# Memory profiling
valgrind --tool=massif target/release/app

# Benchmarking
cargo bench
```

## Anti-Optimization Patterns

### Premature Optimization

```rust
// Don't optimize without measuring
// Start with clear, simple code
fn process(items: &[Item]) -> Vec<Result> {
    items.iter().map(|item| item.process()).collect()
}

// Optimize only hot paths identified by profiling
```

### Over-Optimization

```rust
// Don't sacrifice clarity for minimal gains
// Bad: Unreadable for small benefit
let result = ((((a + b) * c) >> 2) & 0xFF) | 0x80;

// Good: Clear and still fast enough
let result = ((a + b) * c / 4) & 0xFF | 0x80;
```

## Key Principles

1. **Profile first** - Measure before optimizing
2. **Optimize hot paths** - Focus on where time is spent
3. **Reduce allocations** - Allocation is expensive
4. **Use iterators** - Often optimized better than loops
5. **Choose right data structures** - Big O matters
6. **Avoid premature optimization** - Clear code first
7. **Benchmark changes** - Verify improvements
8. **Consider compilation time** - Generics have compile-time cost
