// AMF decoder utilities

// Cursor for AMF decoding
pub struct AMFDecodingCursor {
    /// Current position
    pos: usize,

    // Length
    len: usize,
}

impl AMFDecodingCursor {
    /// Creates new cursor for a buffer
    pub fn new(buffer: &[u8]) -> AMFDecodingCursor {
        AMFDecodingCursor {
            pos: 0,
            len: buffer.len(),
        }
    }

    /// Checks if the cursor position can be incremented by n units
    fn can_increment_pos(&self, n: usize) -> bool {
        let (np, overflow) = self.pos.overflowing_add(n);

        if overflow {
            return false;
        }

        return np <= self.len;
    }

    /// Reads bytes
    /// Errors on buffer overflow
    pub fn read<'a>(&mut self, buffer: &'a [u8], n: usize) -> Result<&'a [u8], ()> {
        if !self.can_increment_pos(n) {
            return Err(());
        }

        let pos = self.pos;
        self.pos += n;

        let r: &'a [u8] = &buffer[pos..(pos + n)];

        Ok(r)
    }

    /// Reads byte
    /// Errors on overflow
    pub fn read_byte(&mut self, buffer: &[u8]) -> Result<u8, ()> {
        let bytes = self.read(buffer, 1)?;

        if let Some(b) = bytes.get(0) {
            Ok(*b)
        } else {
            Err(())
        }
    }

    /// Reads bytes, without changing the cursor
    /// Errors on buffer overflow
    pub fn look<'a>(&self, buffer: &'a [u8], n: usize) -> Result<&'a [u8], ()> {
        if !self.can_increment_pos(n) {
            return Err(());
        }

        let r: &'a [u8] = &buffer[self.pos..(self.pos + n)];

        Ok(r)
    }

    /// Looks byte
    /// Errors on overflow
    pub fn look_byte(&self, buffer: &[u8]) -> Result<u8, ()> {
        let bytes = self.look(buffer, 1)?;

        if let Some(b) = bytes.get(0) {
            Ok(*b)
        } else {
            Err(())
        }
    }

    /// Skips bytes
    pub fn skip(&mut self, n: usize) -> Result<(), ()> {
        if !self.can_increment_pos(n) {
            return Err(());
        }

        self.pos += n;

        Ok(())
    }

    /// Returns true if the cursor is at the end
    pub fn ended(&self) -> bool {
        self.pos >= self.len
    }
}
