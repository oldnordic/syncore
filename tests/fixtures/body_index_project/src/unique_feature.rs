/// Function with unique implementation phrase "cosmic alignment"
pub fn calculate_cosmic_alignment(star_count: usize) -> f64 {
    // This unique phrase should be findable via body search
    let cosmic_alignment_factor = star_count as f64 * 3.14159;
    cosmic_alignment_factor / 2.0
}

/// Another function with overlapping name but different body
pub fn calculate_alignment(items: Vec<String>) -> usize {
    // This has "alignment" in name but different body implementation
    items.iter().filter(|s| s.len() > 5).count()
}

/// Function with database connection phrase
pub fn connect_to_database(url: &str) -> Result<Connection, Error> {
    // Establishes database connection using connection pool
    let pool = ConnectionPool::new(url)?;
    pool.get_connection()
}
