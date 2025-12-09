//! Test fixture for Rust structs, traits, and enums extraction
//! Expected counts:
//! - Structs: 5 (TestStruct, InnerStruct, PublicStruct, GenericStruct<T>, TupleStruct)
//! - Traits: 2 (TestTrait, PublicTrait)
//! - Enums: 2 (TestEnum, PublicEnum)
//! - Impl blocks: 4

/// A basic test struct
struct TestStruct {
    field1: i32,
    field2: String,
}

mod inner_module {
    /// Nested struct
    pub struct InnerStruct {
        data: Vec<i32>,
    }
}

/// Public struct with documentation
#[derive(Debug)]
pub struct PublicStruct<T> {
    pub items: Vec<T>,
    count: usize,
}

/// Generic struct
struct GenericStruct<T: Clone> {
    data: T,
}

/// Tuple struct
struct TupleStruct(String, i32, bool);

/// Basic trait with default methods
trait TestTrait {
    /// Required method
    fn required_method(&self) -> i32;

    /// Default implementation
    fn default_method(&self) -> String {
        "default".to_string()
    }

    /// Another default method
    fn optional_method(&self) -> bool {
        true
    }
}

/// Public trait with generic constraints
pub trait PublicTrait<T: Clone>: Send {
    fn process(&self, item: T) -> T;
    fn validate(&self) -> bool;
}

/// Simple enum
enum TestEnum {
    Variant1,
    Variant2(i32, String),
    Variant3 { x: i32, y: i32 },
}

/// Public enum with variants
#[derive(Debug, Clone)]
pub enum PublicEnum {
    Empty,
    Value(String),
    Multiple { id: u64, name: String },
    Generic(Vec<i32>),
}

// Implementation blocks
impl TestStruct {
    pub fn new(field1: i32, field2: String) -> Self {
        Self { field1, field2 }
    }

    fn get_field1(&self) -> i32 {
        self.field1
    }
}

impl<T: Clone> PublicStruct<T> {
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl TestTrait for TestStruct {
    fn required_method(&self) -> i32 {
        self.field1
    }

    fn default_method(&self) -> String {
        format!("TestStruct: {}", self.field2)
    }
}

impl<T: Clone + Send> PublicTrait<T> for PublicStruct<T> {
    fn process(&self, item: T) -> T {
        item
    }

    fn validate(&self) -> bool {
        self.count > 0
    }
}