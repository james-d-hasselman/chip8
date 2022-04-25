pub trait Keyboard: Send + Sync {
    fn is_key_down(&self, key: u8) -> bool;
    fn get_pressed_key(&self) -> Option<u8>;
}