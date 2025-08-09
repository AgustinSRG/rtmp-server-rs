// Logic to generate unique session IDs

/// Session ID generator
pub struct SessionIdGenerator {
    // Next ID
    pub next_id: u64,
}

impl SessionIdGenerator {
    /// Creates new SessionIdGenerator
    pub fn new() -> SessionIdGenerator {
        SessionIdGenerator { next_id: 1 }
    }

    /// Generates a new unique ID
    pub fn generate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}
