pub struct Sprite {
    bytes: std::vec::Vec<u8>,
}

impl Sprite {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, u8> {
        self.bytes.iter()
    }
}

impl From<&[u8]> for Sprite {
    fn from(s: &[u8]) -> Sprite {
        Sprite {
            bytes: std::vec::Vec::from(s),
        }
    }
}

const X_MAX: usize = 64;
const Y_MAX: usize = 32;

pub trait Display: Send + Sync {
    fn clear(&mut self);
    fn draw(&mut self, x: u8, y: u8, sprite: &Sprite) -> u8;
    fn refresh(&mut self);
}

pub struct ConsoleDisplay {
    buffer: [bool; 2048],
}

fn compute_x_coordinates(x: u8) -> [u16; 8] {
    let mut x_coordinates: [u16; 8] = [0; 8];
    for i in 0..8 as u16 {
        x_coordinates[usize::from(i)] = (u16::from(x) + i) % 64;
    }
    x_coordinates
}

pub fn draw_byte(x: u8, y: u8, byte: u8, display_buffer: &mut [bool]) -> u8 {
    let bits = to_bits(byte);
    let x_coordinates = compute_x_coordinates(x);
    let mut collision = 0;
    for i in 0..8 {
        let vertical_offset = 64 * u16::from(y);
        let coordinate = x_coordinates[i] + vertical_offset;
        let target_bit = &mut display_buffer[usize::from(coordinate)];
        if bits[i] && *target_bit {
            collision = 1;
        }
        *target_bit = *target_bit ^ bits[i];
    }

    return collision;
}

fn to_bits(byte: u8) -> std::vec::Vec<bool> {
    let bits = format!("{:08b}", byte);
    let bits: Vec<bool> = bits
        .chars()
        .map(|c| c.to_digit(10).expect("Memory corrupted, crashing") == 1)
        .collect();
    return bits;
}

impl Display for ConsoleDisplay {
    fn clear(&mut self) {
        self.buffer = [false; 2048];
        self.refresh();
    }

    fn draw(&mut self, x: u8, y: u8, sprite: &Sprite) -> u8 {
        let mut collision = 0;
        for (i, byte) in sprite.iter().enumerate() {
            let i = i as u8;
            let result = draw_byte(x, (y + i) % 32, *byte, &mut self.buffer);
            if result == 1 {
                collision = 1;
            }
        }

        self.refresh();

        return collision;
    }

    fn refresh(&mut self) {
        const Y_RANGE: std::ops::Range<usize> = 0..Y_MAX;
        for y in Y_RANGE {
            let line = &self.buffer[(X_MAX * y)..((X_MAX * y) + X_MAX)];
            for (_, pixel) in line.iter().enumerate() {
                if *pixel {
                    print!("*");
                } else {
                    print!(" ");
                }
            }
            println!();
        }
    }
}



#[cfg(test)]
mod tests {
    use crate::graphics::compute_x_coordinates;
    use crate::graphics::draw_byte;
    use crate::graphics::ConsoleDisplay;
    use crate::graphics::Display;
    use crate::graphics::Sprite;

    #[test]
    fn test_console_display_clear() {
        let mut x = ConsoleDisplay {
            buffer: [true; 2048],
        };
        x.clear();
        assert_eq!(x.buffer, [false; 2048]);
    }
    //x.draw(x: u8, y: u8, sprite: &Sprite)

    #[test]
    fn test_draw_byte() {
        let mut display_buffer = [false; 2048];
        draw_byte(0, 0, 0xFF, &mut display_buffer);
        let mut expected_display_buffer = [false; 2048];
        expected_display_buffer[0..8].clone_from_slice(&[true; 8]);
        assert_eq!(display_buffer, expected_display_buffer);
    }

    #[test]
    fn test_draw_byte_horizontal_wrap() {
        let mut display_buffer = [false; 64];
        draw_byte(60, 0, 0xFF, &mut display_buffer);
        let mut expected_display_buffer = [false; 64];
        expected_display_buffer[60..64].clone_from_slice(&[true; 4]);
        expected_display_buffer[0..4].clone_from_slice(&[true; 4]);
        assert_eq!(display_buffer, expected_display_buffer);
    }

    #[test]
    fn test_compute_x_coordinates() {
        let x_coordinates = compute_x_coordinates(60);
        let expected_x_coordinates = [60, 61, 62, 63, 0, 1, 2, 3];
        assert_eq!(x_coordinates, expected_x_coordinates);
    }

    #[test]
    fn test_draw_vertical_wrap() {
        let mut display = ConsoleDisplay {
            buffer: [false; 2048],
        };
        let bytes = [0xFF, 0xFF];
        let sprite = Sprite::from(&bytes[..]);
        display.draw(55, 31, &sprite);
        let mut expected_buffer = [false; 2048];
        expected_buffer[55..63].clone_from_slice(&[true; 8]);
        expected_buffer[2039..2047].clone_from_slice(&[true; 8]);
        assert_eq!(display.buffer, expected_buffer);
    }
}
