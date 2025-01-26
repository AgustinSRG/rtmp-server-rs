// AMF decoder

// Cursor for AMF decoding
pub struct AMFDecodingCursor {
    /// Current position
    pos: usize,

    // Length
    len: usize,
}

impl AMFDecodingCursor {
    /// Reads bytes
    pub fn read<'a>(&mut self, buffer: &'a[u8], n: usize) -> Result<&'a [u8], ()> {
        if self.pos + n > self.len {
            return Err(())
        }

        let pos = self.pos;
        self.pos += n;

        let r: &'a [u8] = &buffer[pos..(pos + n)];

        Ok(r)
    }

    /// Reads bytes, without changing the cursor
    pub fn look<'a>(&self, buffer: &'a[u8], n: usize) -> Result<&'a [u8], ()> {
        if self.pos + n > self.len {
            return Err(())
        }

        let r: &'a [u8] = &buffer[self.pos..(self.pos + n)];

        Ok(r)
    }

    /// Skips bytes
    pub fn skip(&mut self, n: usize) -> Result<(), ()> {
        if self.pos + n > self.len {
            return Err(())
        }

        self.pos += n;

        Ok(())
    }

    /// Returns true if the cursor is at the end
    pub fn ended(&self) -> bool {
        self.pos >= self.len
    }
}



