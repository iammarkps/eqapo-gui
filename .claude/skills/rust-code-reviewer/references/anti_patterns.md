# Rust Anti-Patterns

Common patterns to avoid in Rust code.

## Ownership Anti-Patterns

### Unnecessary Cloning
```rust
// Bad: Cloning when borrowing suffices
fn process(items: Vec<String>) {
    for item in items.clone() {
        println!("{}", item);
    }
}

// Good: Borrow instead
fn process(items: &[String]) {
    for item in items {
        println!("{}", item);
    }
}
```

### Clone to Satisfy Borrow Checker
```rust
// Bad: Clone to avoid thinking about lifetimes
fn get_first(items: &Vec<String>) -> String {
    items[0].clone()
}

// Good: Return reference with proper lifetime
fn get_first(items: &[String]) -> &str {
    &items[0]
}
```

### Taking Ownership Unnecessarily
```rust
// Bad: Takes ownership when reference would work
fn calculate_length(s: String) -> usize {
    s.len()
}

// Good: Accept reference
fn calculate_length(s: &str) -> usize {
    s.len()
}
```

## Error Handling Anti-Patterns

### Unwrap in Production Code
```rust
// Bad: Panics on error
fn load_config() -> Config {
    let content = std::fs::read_to_string("config.toml").unwrap();
    toml::from_str(&content).unwrap()
}

// Good: Propagate errors
fn load_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;
    Ok(toml::from_str(&content)?)
}
```

### Ignoring Results
```rust
// Bad: Silently ignoring errors
file.write_all(data);

// Good: Handle the result
file.write_all(data)?;
// Or explicitly ignore with reasoning
let _ = file.write_all(data); // Best effort, don't care if fails
```

### Using Strings as Errors
```rust
// Bad: String errors lose context
fn parse_config(s: &str) -> Result<Config, String> {
    Err("parse failed".to_string())
}

// Good: Use proper error types
#[derive(Debug, Error)]
enum ConfigError {
    #[error("Parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },
}
```

## Performance Anti-Patterns

### Allocating in Loops
```rust
// Bad: Repeated allocations
for i in 0..1000 {
    let s = format!("Item {}", i);
    process(&s);
}

// Good: Reuse buffer
let mut buffer = String::new();
for i in 0..1000 {
    buffer.clear();
    write!(&mut buffer, "Item {}", i).unwrap();
    process(&buffer);
}
```

### Unnecessary Collecting
```rust
// Bad: Intermediate collection
let sum: i32 = numbers
    .iter()
    .filter(|&&x| x > 0)
    .collect::<Vec<_>>()
    .iter()
    .sum();

// Good: Direct iterator chain
let sum: i32 = numbers
    .iter()
    .filter(|&&x| x > 0)
    .sum();
```

### Using to_string() Unnecessarily
```rust
// Bad: Allocates when to_owned() would work
let s: String = "hello".to_string();

// Good: Use to_owned() for &str -> String
let s: String = "hello".to_owned();

// Or even better, keep as &str if possible
let s: &str = "hello";
```

## API Design Anti-Patterns

### Public Struct Fields
```rust
// Bad: No encapsulation, can't evolve
pub struct User {
    pub name: String,
    pub email: String,
}

// Good: Use accessors
pub struct User {
    name: String,
    email: String,
}

impl User {
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn set_email(&mut self, email: String) -> Result<(), ValidationError> {
        validate_email(&email)?;
        self.email = email;
        Ok(())
    }
}
```

### Stringly-Typed APIs
```rust
// Bad: Using strings for known values
fn set_log_level(level: &str) {
    match level {
        "debug" => { },
        "info" => { },
        _ => panic!("invalid level"),
    }
}

// Good: Use enums
enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

fn set_log_level(level: LogLevel) { }
```

### Unclear Lifetimes
```rust
// Bad: Unclear lifetime relationship
fn get_value<'a, 'b>(map: &'a HashMap<String, String>, key: &'b str) -> &'a str {
    map.get(key).map(|s| s.as_str()).unwrap_or("")
}

// Good: Make relationship explicit
fn get_value<'a>(map: &'a HashMap<String, String>, key: &str) -> &'a str {
    map.get(key).map(|s| s.as_str()).unwrap_or("")
}
```

## Concurrency Anti-Patterns

### Sharing Mutex-Wrapped Data
```rust
// Bad: Cloning Arc for each thread
let counter = Arc::new(Mutex::new(0));
for _ in 0..10 {
    let counter = counter.clone(); // Easy to forget
    thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    });
}

// Better: Make Arc explicit in type
struct Counter {
    inner: Arc<Mutex<i32>>,
}

impl Counter {
    fn increment(&self) {
        let mut num = self.inner.lock().unwrap();
        *num += 1;
    }
}
```

### Holding Locks Too Long
```rust
// Bad: Holding lock while doing I/O
let data = mutex.lock().unwrap();
expensive_operation(&data);
write_to_disk(&data); // Still holding lock!

// Good: Minimize critical section
let data = {
    let guard = mutex.lock().unwrap();
    guard.clone() // Or extract needed data
};
expensive_operation(&data);
write_to_disk(&data);
```

### Deadlock Patterns
```rust
// Bad: Nested locks can deadlock
fn transfer(from: &Mutex<Account>, to: &Mutex<Account>, amount: u64) {
    let mut from_account = from.lock().unwrap();
    let mut to_account = to.lock().unwrap(); // Deadlock if called both ways!
    from_account.balance -= amount;
    to_account.balance += amount;
}

// Good: Lock in consistent order
fn transfer(from: &Mutex<Account>, to: &Mutex<Account>, amount: u64) {
    let (first, second) = if from as *const _ < to as *const _ {
        (from, to)
    } else {
        (to, from)
    };
    let mut first_account = first.lock().unwrap();
    let mut second_account = second.lock().unwrap();
    // Transfer logic
}
```

## Testing Anti-Patterns

### Over-Reliance on Mocking
```rust
// Bad: Testing implementation details
#[test]
fn test_implementation() {
    let mock = MockDatabase::new();
    mock.expect_query().times(1).return_once(|| Ok(data));
    service.process(&mock);
}

// Good: Test behavior
#[test]
fn test_behavior() {
    let service = Service::new(test_database());
    let result = service.process();
    assert_eq!(result, expected);
}
```

### Not Testing Error Cases
```rust
// Bad: Only happy path
#[test]
fn test_parse() {
    assert_eq!(parse("123"), Ok(123));
}

// Good: Test error cases too
#[test]
fn test_parse_invalid() {
    assert!(parse("not a number").is_err());
    assert!(parse("").is_err());
}
```

## Macro Anti-Patterns

### Over-Using Macros
```rust
// Bad: Macro when function would work
macro_rules! double {
    ($x:expr) => { $x * 2 }
}

// Good: Use function
fn double(x: i32) -> i32 {
    x * 2
}
```

### Unclear Macro Hygiene
```rust
// Bad: Macro that captures variables
macro_rules! log_value {
    ($x:expr) => {
        let name = stringify!($x);
        println!("{} = {}", name, $x);
    }
}

// Can conflict with existing `name` variable
```

## Type System Anti-Patterns

### Excessive Generics
```rust
// Bad: Over-generic for no benefit
fn process<T, U, V>(a: T, b: U, c: V) 
where 
    T: AsRef<str>,
    U: AsRef<str>,
    V: AsRef<str>
{
    // All just strings...
}

// Good: Use concrete types when appropriate
fn process(a: &str, b: &str, c: &str) { }
```

### Ignoring Type Safety
```rust
// Bad: Using raw pointers when safe abstractions exist
unsafe {
    let ptr = &value as *const T;
    let data = *ptr;
}

// Good: Use safe abstractions
let data = value;
```

## Documentation Anti-Patterns

### Missing Safety Documentation
```rust
// Bad: Unsafe without documentation
pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> &[u8] {
    std::slice::from_raw_parts(ptr, len)
}

// Good: Document safety requirements
/// # Safety
/// 
/// `ptr` must be valid for reads of `len * size_of::<u8>()` bytes.
/// `ptr` must be non-null and aligned.
/// The memory must not be mutated for the duration of the returned slice.
pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> &[u8] {
    std::slice::from_raw_parts(ptr, len)
}
```

### Outdated Comments
```rust
// Bad: Comment doesn't match code
// Returns the sum of two numbers
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

// Good: Keep comments accurate or remove them
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
```
