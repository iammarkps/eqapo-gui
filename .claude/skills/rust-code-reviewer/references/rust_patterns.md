# Rust Idiomatic Patterns

Common patterns that represent idiomatic Rust code.

## Ownership Patterns

### Accepting Borrowed Data
```rust
// Good: Accept &str for maximum flexibility
fn process_text(text: &str) { }

// Less flexible: Requires String allocation
fn process_text(text: &String) { }
```

### Returning Owned vs Borrowed
```rust
// Return owned when data is created
fn create_message() -> String {
    format!("Hello, world!")
}

// Return borrowed for existing data
fn get_name(&self) -> &str {
    &self.name
}
```

### Clone-on-Write (Cow)
```rust
use std::borrow::Cow;

fn process<'a>(input: &'a str, uppercase: bool) -> Cow<'a, str> {
    if uppercase {
        Cow::Owned(input.to_uppercase())
    } else {
        Cow::Borrowed(input)
    }
}
```

## Error Handling Patterns

### Custom Error Types with thiserror
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid format: {0}")]
    Parse(String),
}
```

### Result Composition
```rust
// Chain operations with ?
fn load_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;
    let config = toml::from_str(&content)
        .map_err(|e| ConfigError::Parse(e.to_string()))?;
    Ok(config)
}
```

### Option to Result Conversion
```rust
// Convert None to custom error
let value = map.get("key")
    .ok_or_else(|| Error::MissingKey("key".into()))?;
```

## Builder Pattern

```rust
pub struct Config {
    host: String,
    port: u16,
    timeout: Duration,
}

pub struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    timeout: Option<Duration>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            timeout: None,
        }
    }
    
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    
    pub fn build(self) -> Result<Config, BuildError> {
        Ok(Config {
            host: self.host.ok_or(BuildError::MissingHost)?,
            port: self.port.unwrap_or(8080),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}
```

## Type-State Pattern

```rust
// Encode state in the type system
pub struct Locked;
pub struct Unlocked;

pub struct Door<State = Locked> {
    _state: PhantomData<State>,
}

impl Door<Locked> {
    pub fn new() -> Self {
        Door { _state: PhantomData }
    }
    
    pub fn unlock(self, key: &Key) -> Door<Unlocked> {
        // Validation logic
        Door { _state: PhantomData }
    }
}

impl Door<Unlocked> {
    pub fn open(&mut self) {
        // Can only open unlocked door
    }
}
```

## Iterator Patterns

### Custom Iterator
```rust
struct Counter {
    current: u32,
    max: u32,
}

impl Iterator for Counter {
    type Item = u32;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.max {
            let result = self.current;
            self.current += 1;
            Some(result)
        } else {
            None
        }
    }
}
```

### Chainable Processing
```rust
let result: Vec<_> = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.process())
    .collect();
```

## Newtype Pattern

```rust
// Type safety through newtypes
pub struct UserId(u64);
pub struct PostId(u64);

// Prevents mixing up IDs
fn get_user(id: UserId) -> User { }
fn get_post(id: PostId) -> Post { }
```

## Extension Trait Pattern

```rust
// Extend existing types
pub trait StringExt {
    fn truncate_to(&self, max_len: usize) -> String;
}

impl StringExt for str {
    fn truncate_to(&self, max_len: usize) -> String {
        if self.len() <= max_len {
            self.to_string()
        } else {
            format!("{}...", &self[..max_len])
        }
    }
}
```

## From/Into Pattern

```rust
// Prefer implementing From, get Into for free
impl From<String> for UserId {
    fn from(s: String) -> Self {
        UserId(s.parse().unwrap_or(0))
    }
}

// Now both work:
let id: UserId = string.into();
let id = UserId::from(string);
```

## RAII Pattern

```rust
// Resource management through Drop
pub struct FileGuard {
    file: File,
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        // Cleanup happens automatically
        self.file.sync_all().ok();
    }
}
```

## Interior Mutability

```rust
use std::cell::RefCell;

pub struct Cache {
    data: RefCell<HashMap<String, Value>>,
}

impl Cache {
    pub fn get(&self, key: &str) -> Option<Value> {
        // Can mutate through immutable reference
        self.data.borrow_mut().get(key).cloned()
    }
}
```

## Visitor Pattern

```rust
pub trait Visitor {
    fn visit_string(&mut self, s: &str);
    fn visit_number(&mut self, n: i64);
}

pub enum Value {
    String(String),
    Number(i64),
}

impl Value {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        match self {
            Value::String(s) => visitor.visit_string(s),
            Value::Number(n) => visitor.visit_number(n),
        }
    }
}
```
