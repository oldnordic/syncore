// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP C: Entity type boosting for fusion query relevance
//
// Boosts Function/Struct/Impl entities and de-boosts Import entities
// to improve code search relevance.

/// Entity categories for boost calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCategory {
    /// Primary implementation entities (Function, Struct, Class, Impl, Method)
    /// Boost: 1.3-1.4x
    Implementation,
    /// Secondary type entities (Trait, Enum, Interface, TypeAlias)
    /// Boost: 1.15-1.2x
    Secondary,
    /// Import/Use statements
    /// De-boost: 0.6-0.7x
    Import,
    /// Neutral entities (Module, Package, etc.)
    /// Boost: 1.0x
    Neutral,
}

impl EntityCategory {
    /// Classify an entity kind into a category
    pub fn from_kind(kind: &str) -> Self {
        let kind_lower = kind.to_lowercase();

        match kind_lower.as_str() {
            // Primary implementation entities
            "function" | "fn" | "def" | "func" => Self::Implementation,
            "struct" | "structure" => Self::Implementation,
            "class" => Self::Implementation,
            "impl" | "implementation" => Self::Implementation,
            "method" | "member_function" => Self::Implementation,

            // Secondary type entities
            "trait" | "interface" | "protocol" => Self::Secondary,
            "enum" | "enumeration" => Self::Secondary,
            "typealias" | "type_alias" | "typedef" => Self::Secondary,
            "constant" | "const" | "static" => Self::Secondary,

            // Import statements (de-boost)
            "import" | "use" | "require" | "include" => Self::Import,
            "extern_crate" | "extern" => Self::Import,

            // Everything else is neutral
            _ => Self::Neutral,
        }
    }

    /// Get the boost multiplier for this category
    pub fn boost_multiplier(&self) -> f32 {
        match self {
            Self::Implementation => 1.35, // Strong boost for implementations
            Self::Secondary => 1.18,      // Moderate boost for types
            Self::Import => 0.65,         // De-boost imports
            Self::Neutral => 1.0,         // No change for neutral
        }
    }
}

/// Compute boost multiplier for an entity type
///
/// # Arguments
/// * `kind` - The entity kind string (e.g., "Function", "Import", "Struct")
///
/// # Returns
/// Boost multiplier:
/// - > 1.0: Entity is boosted (implementations, types)
/// - < 1.0: Entity is de-boosted (imports)
/// - = 1.0: Neutral (modules, unknowns)
///
/// # Examples
/// ```
/// use syncore::code_graph::entity_boost::compute_entity_type_boost;
///
/// assert!(compute_entity_type_boost("Function") > 1.0);
/// assert!(compute_entity_type_boost("Import") < 1.0);
/// assert!((compute_entity_type_boost("Module") - 1.0).abs() < 0.01);
/// ```
pub fn compute_entity_type_boost(kind: &str) -> f32 {
    EntityCategory::from_kind(kind).boost_multiplier()
}

/// Apply entity type boost to a combined score
///
/// # Arguments
/// * `score` - The base combined score (0.0 to 1.0)
/// * `entity_kind` - The entity kind string
///
/// # Returns
/// Boosted score, clamped to [0.0, 1.0]
pub fn apply_entity_boost(score: f32, entity_kind: &str) -> f32 {
    let boost = compute_entity_type_boost(entity_kind);
    (score * boost).clamp(0.0, 1.0)
}

/// Compute body snippet boost multiplier (APEX v1.7 Phase 4)
///
/// Entities with body_snippet get a boost because they provide more semantic context
/// for matching user queries against implementation details.
///
/// # Arguments
/// * `has_body` - Whether the entity has a body_snippet
///
/// # Returns
/// Boost multiplier:
/// - 1.15: Entity has body_snippet (more semantic context)
/// - 1.0: Entity has no body_snippet (neutral)
///
/// # Examples
/// ```
/// use syncore::code_graph::entity_boost::compute_body_boost;
///
/// assert!(compute_body_boost(true) > 1.0);
/// assert!((compute_body_boost(false) - 1.0).abs() < 0.01);
/// ```
pub fn compute_body_boost(has_body: bool) -> f32 {
    if has_body {
        1.15 // Moderate boost for having body content
    } else {
        1.0 // Neutral for entities without body
    }
}

/// Apply combined entity type + body boost to a score (APEX v1.7 Phase 4)
///
/// # Arguments
/// * `score` - The base combined score (0.0 to 1.0)
/// * `entity_kind` - The entity kind string
/// * `has_body` - Whether the entity has a body_snippet
///
/// # Returns
/// Boosted score with both type and body multipliers applied, clamped to [0.0, 1.0]
pub fn apply_combined_boost(score: f32, entity_kind: &str, has_body: bool) -> f32 {
    let type_boost = compute_entity_type_boost(entity_kind);
    let body_boost = compute_body_boost(has_body);
    (score * type_boost * body_boost).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_category() {
        assert_eq!(
            EntityCategory::from_kind("function"),
            EntityCategory::Implementation
        );
        assert_eq!(
            EntityCategory::from_kind("struct"),
            EntityCategory::Implementation
        );
        assert_eq!(
            EntityCategory::from_kind("class"),
            EntityCategory::Implementation
        );
    }

    #[test]
    fn test_import_category() {
        assert_eq!(EntityCategory::from_kind("import"), EntityCategory::Import);
        assert_eq!(EntityCategory::from_kind("use"), EntityCategory::Import);
    }

    #[test]
    fn test_boost_values() {
        assert!(EntityCategory::Implementation.boost_multiplier() > 1.0);
        assert!(EntityCategory::Import.boost_multiplier() < 1.0);
        assert!((EntityCategory::Neutral.boost_multiplier() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_body_boost() {
        // APEX v1.7 Phase 4: Test body_snippet boost
        assert!(compute_body_boost(true) > 1.0);
        assert!((compute_body_boost(false) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_combined_boost() {
        // APEX v1.7 Phase 4: Test combined type + body boost
        let base_score = 0.6; // Use 0.6 to avoid clamping to 1.0

        // Function with body should get both boosts
        let boosted = apply_combined_boost(base_score, "function", true);
        assert!(boosted > base_score);

        // Function without body should only get type boost
        let type_only = apply_combined_boost(base_score, "function", false);
        assert!(type_only > base_score);
        assert!(
            boosted > type_only,
            "Body adds extra boost: {} vs {}",
            boosted,
            type_only
        );

        // Import with body (unlikely but possible)
        let import_boost = apply_combined_boost(base_score, "import", true);
        assert!(import_boost < base_score); // Import de-boost dominates
    }
}
