pub struct Crc32 {
    table: [u32; 256],
}

impl Crc32 {
    pub fn new() -> Self {
        let mut table = [0u32; 256];
        for (i, item) in table.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 == 1 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
            *item = crc;
        }
        Self { table }
    }

    pub fn checksum(&self, buffer: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in buffer {
            let idx = ((crc & 0xFF) ^ byte as u32) as usize;
            crc = (crc >> 8) ^ self.table[idx];
        }
        crc ^ 0xFFFF_FFFF
    }
}
