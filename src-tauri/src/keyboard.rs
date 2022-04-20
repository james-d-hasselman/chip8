pub trait Keyboard: Send + Sync {
    fn is_key_down(&self, key: u8) -> bool;
    fn get_pressed_key(&self) -> Option<u8>;
}

/*pub struct Keyboard {
    keys: HashMap<u8, bool>,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            keys: HashMap::from([
                (0, false),
                (1, false),
                (2, false),
                (3, false),
                (4, false),
                (5, false),
                (6, false),
                (7, false),
                (8, false),
                (9, false),
                (10, false),
                (11, false),
                (12, false),
                (13, false),
                (14, false),
                (15, false),
            ]),
        }
    }

    /*pub fn from(keyboard_state: &KeyboardState) -> Self {
        Self {
            keys: keyboard_state
                .scancodes()
                .filter_map(|code| match code {
                    (
                        sc
                        @
                        (Scancode::Num1
                        | Scancode::Num2
                        | Scancode::Num3
                        | Scancode::Num4
                        | Scancode::Q
                        | Scancode::W
                        | Scancode::E
                        | Scancode::R
                        | Scancode::A
                        | Scancode::S
                        | Scancode::D
                        | Scancode::F
                        | Scancode::Z
                        | Scancode::X
                        | Scancode::C
                        | Scancode::V),
                        state,
                    ) => Some((
                        Keyboard::to_keycode(sc).expect("Unexpected scancode"),
                        state,
                    )),
                    _ => None,
                })
                .into_iter()
                .collect(),
        }
    }

    fn to_keycode(code: sdl2::keyboard::Scancode) -> Option<u8> {
        match code {
            Scancode::Num1 => Some(1),
            Scancode::Num2 => Some(2),
            Scancode::Num3 => Some(3),
            Scancode::Num4 => Some(0xC),
            Scancode::Q => Some(4),
            Scancode::W => Some(5),
            Scancode::E => Some(6),
            Scancode::R => Some(0xD),
            Scancode::A => Some(7),
            Scancode::S => Some(8),
            Scancode::D => Some(9),
            Scancode::F => Some(0xE),
            Scancode::Z => Some(0xA),
            Scancode::X => Some(0),
            Scancode::C => Some(0xB),
            Scancode::V => Some(0xF),
            _ => None,
        }
    }*/

    pub fn is_key_down(&self, key: u8) -> bool {
        *self.keys.get(&key).expect("Unexpected key. Crashing.")
    }

    pub fn get_pressed_key(&self) -> Option<u8> {
        match self.keys.iter().find(|(_, &s)| s) {
            Some((k, _)) => Some(*k),
            _ => None,
        }
    }
}*/
