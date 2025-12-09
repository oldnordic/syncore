//! Test fixture for Rust functions extraction
//! Expected counts:
//! - Free functions: 5 (free_function, async_function, generic_function, private_function, another_free)
//! - Methods in impls: 8 (4 in TestMethods, 2 in AnotherClass, 1 in TraitImpl, 1 in DefaultImpl)
//! - Total functions: 13

/// Free function with documentation
pub fn free_function(param1: String, param2: i32) -> Result<String, Error> {
    Ok(format!("{}: {}", param1, param2))
}

/// Async function
pub async fn async_function(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = reqwest::get(url).await?;
    Ok(response.bytes().await?.to_vec())
}

/// Generic function
fn generic_function<T: Clone + std::fmt::Debug>(data: &T) -> T
where
    T: Default,
{
    let cloned = data.clone();
    println!("Cloned data: {:?}", cloned);
    cloned
}

/// Private function with different parameter types
fn private_function(
    name: &str,
    age: u32,
    active: bool,
    tags: Vec<String>,
) -> Option<usize> {
    if active {
        Some(name.len())
    } else {
        None
    }
}

/// Another free function with complex return type
pub fn another_free() -> std::collections::HashMap<String, std::time::SystemTime> {
    std::collections::HashMap::new()
}

// Class with methods
struct TestMethods {
    value: i32,
    name: String,
}

impl TestMethods {
    /// Constructor
    pub fn new(value: i32, name: String) -> Self {
        Self { value, name }
    }

    /// Public method
    pub fn get_value(&self) -> i32 {
        self.value
    }

    /// Private method
    fn internal_logic(&self) -> bool {
        self.value > 0
    }

    /// Static method
    pub fn create_default() -> Self {
        Self {
            value: 42,
            name: "default".to_string(),
        }
    }
}

// Another class for more method testing
pub struct AnotherClass {
    data: Vec<String>,
}

impl AnotherClass {
    pub fn add_item(&mut self, item: String) {
        self.data.push(item);
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }
}

// Trait implementation
trait TraitImpl {
    fn trait_method(&self) -> String;
    fn default_trait_method(&self) -> i32 {
        42
    }
}

impl TraitImpl for TestMethods {
    fn trait_method(&self) -> String {
        self.name.clone()
    }
}

// Default trait implementation
impl Default for AnotherClass {
    fn default() -> Self {
        Self {
            data: Vec::new(),
        }
    }
}