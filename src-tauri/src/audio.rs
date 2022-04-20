pub trait Buzzer: Send + Sync {
    fn initialize(self, frequency: f32, volume: f32);
    fn play(&self);
    fn pause(&self);
}
