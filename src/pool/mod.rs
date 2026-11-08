use std::collections::VecDeque;

pub struct ConnectionPool { pool: VecDeque<String>, max_size: usize }
impl ConnectionPool {
    pub fn new(max_size: usize) -> Self { Self { pool: VecDeque::with_capacity(max_size), max_size } }
    pub fn acquire(&mut self) -> Option<String> { self.pool.pop_front() }
    pub fn release(&mut self, conn: String) { if self.pool.len() < self.max_size { self.pool.push_back(conn); } }
}
