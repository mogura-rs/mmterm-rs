use std::collections::HashMap;

pub struct Canvas {
    grid: HashMap<(i32, i32), u8>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl Canvas {
    pub fn new() -> Self {
        Canvas {
            grid: HashMap::new(),
            min_x: i32::MAX,
            max_x: i32::MIN,
            min_y: i32::MAX,
            max_y: i32::MIN,
        }
    }

    pub fn clear(&mut self) {
        self.grid.clear();
        self.min_x = i32::MAX;
        self.max_x = i32::MIN;
        self.min_y = i32::MAX;
        self.max_y = i32::MIN;
    }

    fn get_pixel_map(x: i32, y: i32) -> ((i32, i32), u8) {
        let char_x = x / 2;
        let char_y = y / 4;

        let sub_x = x % 2;
        let sub_y = y % 4;

        // Ensure positive modulus for negative coordinates
        let sub_x = if sub_x < 0 { sub_x + 2 } else { sub_x };
        let sub_y = if sub_y < 0 { sub_y + 4 } else { sub_y };

        // Adjust char coordinate for negative inputs if needed
        // Integer division in Rust truncates towards zero, so -1 / 2 = 0.
        // We need floor division.
        let char_x = if x < 0 && x % 2 != 0 { char_x - 1 } else { char_x };
        let char_y = if y < 0 && y % 4 != 0 { char_y - 1 } else { char_y };

        let mask = match (sub_x, sub_y) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (0, 3) => 0x40,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (1, 3) => 0x80,
            _ => 0,
        };

        ((char_x, char_y), mask)
    }

    pub fn set(&mut self, x: f32, y: f32) {
        let ix = x.round() as i32;
        let iy = y.round() as i32;

        let ((cx, cy), mask) = Self::get_pixel_map(ix, iy);

        *self.grid.entry((cx, cy)).or_insert(0) |= mask;

        if cx < self.min_x { self.min_x = cx; }
        if cx > self.max_x { self.max_x = cx; }
        if cy < self.min_y { self.min_y = cy; }
        if cy > self.max_y { self.max_y = cy; }
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let x1 = x1.round() as i32;
        let y1 = y1.round() as i32;
        let x2 = x2.round() as i32;
        let y2 = y2.round() as i32;

        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x1;
        let mut y = y1;

        loop {
            self.set(x as f32, y as f32);
            if x == x2 && y == y2 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn frame(&self) -> String {
        if self.grid.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        for y in self.min_y..=self.max_y {
            for x in self.min_x..=self.max_x {
                if let Some(&mask) = self.grid.get(&(x, y)) {
                    // Braille starts at U+2800
                    let c = std::char::from_u32(0x2800 + mask as u32).unwrap_or(' ');
                    output.push(c);
                } else {
                    output.push(' '); // Or appropriate empty character, usually space (U+2800 is blank braille pattern but space is better for terminal copy paste)
                    // Actually, U+2800 is empty pattern. Space is space.
                    // If we use U+2800 it looks like a space but might have different width properties in some fonts? Usually it's fine.
                    // Let's use space ' ' for empty cells to minimize artifacting.
                }
            }
            output.push_str("\r\n");
        }
        output
    }
}
