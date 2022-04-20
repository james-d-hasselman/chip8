use std::{time::Duration, sync::Arc};

pub trait Timer {
    fn start(&mut self, interval: Duration, callback: Box<dyn FnMut()>);
    fn stop(self);
}
